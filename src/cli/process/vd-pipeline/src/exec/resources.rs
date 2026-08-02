//! In-job resource class leases (GPU / CPU / …).
//!
//! Parallel workflow branches may fan out up to `max_parallel`, but contended
//! classes (e.g. `metal_gpu` capacity 1) serialize leaf invokes that need them.
//! Without this gate, meeting Jobs with several ASR branches open multiple
//! Metal contexts and fail with buffer allocation errors.

use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex};
use std::thread;

use crate::job::{ArgValue, Capability, Job, ResolvedStep, Step};

/// Shared pool for one Executor::run.
pub struct ResourcePool {
    capacity: BTreeMap<String, u32>,
    state: Mutex<BTreeMap<String, u32>>,
    cv: Condvar,
}

/// RAII lease — releases on drop (including panic unwind).
pub struct ResourceLease<'a> {
    pool: &'a ResourcePool,
    need: BTreeMap<String, u32>,
}

impl Drop for ResourceLease<'_> {
    fn drop(&mut self) {
        self.pool.release(&self.need);
    }
}

impl ResourcePool {
    pub fn new(capacity: BTreeMap<String, u32>) -> Self {
        Self {
            capacity,
            state: Mutex::new(BTreeMap::new()),
            cv: Condvar::new(),
        }
    }

    /// Block until `need` fits under capacity, then lease.
    pub fn acquire(&self, need: &BTreeMap<String, u32>) -> ResourceLease<'_> {
        if need.is_empty() {
            return ResourceLease {
                pool: self,
                need: BTreeMap::new(),
            };
        }
        let mut leased = self.state.lock().unwrap();
        loop {
            if can_lease(&self.capacity, &leased, need) {
                for (k, n) in need {
                    *leased.entry(k.clone()).or_insert(0) += *n;
                }
                return ResourceLease {
                    pool: self,
                    need: need.clone(),
                };
            }
            leased = self.cv.wait(leased).unwrap();
        }
    }

    fn release(&self, need: &BTreeMap<String, u32>) {
        if need.is_empty() {
            return;
        }
        let mut leased = self.state.lock().unwrap();
        for (k, n) in need {
            if let Some(v) = leased.get_mut(k) {
                *v = v.saturating_sub(*n);
            }
        }
        self.cv.notify_all();
    }
}

fn can_lease(
    capacity: &BTreeMap<String, u32>,
    leased: &BTreeMap<String, u32>,
    need: &BTreeMap<String, u32>,
) -> bool {
    for (k, n) in need {
        let cap = effective_capacity(capacity, k);
        let used = leased.get(k).copied().unwrap_or(0);
        if used.saturating_add(*n) > cap {
            return false;
        }
    }
    true
}

fn effective_capacity(capacity: &BTreeMap<String, u32>, class: &str) -> u32 {
    if let Some(c) = capacity.get(class) {
        return *c;
    }
    // Contended accelerators default to a single slot when Job omitted the class.
    if is_accelerator_class(class) {
        1
    } else {
        // Unknown non-accelerator: do not block (cpu already declared in defaults).
        u32::MAX
    }
}

fn is_accelerator_class(class: &str) -> bool {
    class == "gpu"
        || class == "metal_gpu"
        || class == "cuda_gpu"
        || class.ends_with("_gpu")
}

/// Job-level caps: explicit `job.resources` overlay platform defaults.
pub fn resolve_job_capacity(job: &Job) -> BTreeMap<String, u32> {
    let mut caps = default_capacity();
    for (k, v) in &job.resources {
        caps.insert(k.clone(), *v);
    }
    caps
}

fn default_capacity() -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    let cpu = thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    m.insert("cpu".into(), cpu.max(1));
    #[cfg(target_os = "macos")]
    {
        m.insert("metal_gpu".into(), 1);
    }
    m
}

/// Units this leaf must hold for the duration of `binder.invoke`.
pub fn step_resource_need(step: &ResolvedStep, job_step: &Step) -> BTreeMap<String, u32> {
    let mut need = BTreeMap::new();
    if let Some(class) = job_step.resource.as_deref() {
        need.insert(class.to_string(), 1);
        return need;
    }
    let class = default_resource_class(step.capability, &step.options);
    need.insert(class.into(), 1);
    need
}

fn default_resource_class(
    capability: Capability,
    options: &BTreeMap<String, ArgValue>,
) -> &'static str {
    match capability {
        Capability::Transcribe | Capability::Diarize => {
            match options
                .get("device")
                .and_then(ArgValue::as_string)
                .as_deref()
            {
                Some("cpu") => "cpu",
                Some("cuda") => "cuda_gpu",
                Some("metal") => "metal_gpu",
                // Unset / auto: prefer Metal on macOS (matches default_job device).
                Some("auto") | None => {
                    #[cfg(target_os = "macos")]
                    {
                        "metal_gpu"
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        "cpu"
                    }
                }
                _ => "cpu",
            }
        }
        _ => "cpu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn accelerator_capacity_defaults_to_one() {
        let pool = ResourcePool::new(BTreeMap::new());
        let need = BTreeMap::from([("metal_gpu".into(), 1u32)]);
        let _a = pool.acquire(&need);
        // Second acquire would block — verified in integration via timed parallel test.
        assert!(can_lease(&pool.capacity, &pool.state.lock().unwrap(), &need) == false);
    }

    #[test]
    fn serializes_contended_class() {
        let pool = Arc::new(ResourcePool::new(BTreeMap::from([(
            "metal_gpu".into(),
            1u32,
        )])));
        let concurrent = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let need = BTreeMap::from([("metal_gpu".into(), 1u32)]);

        thread::scope(|scope| {
            for _ in 0..3 {
                let pool = Arc::clone(&pool);
                let concurrent = Arc::clone(&concurrent);
                let peak = Arc::clone(&peak);
                let need = need.clone();
                scope.spawn(move || {
                    let _lease = pool.acquire(&need);
                    let now = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(30));
                    concurrent.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        assert_eq!(peak.load(Ordering::SeqCst), 1);
    }
}
