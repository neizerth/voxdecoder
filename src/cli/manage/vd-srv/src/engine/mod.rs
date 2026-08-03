//! In-process execution engine (store + scheduler + workers).

mod observe;
mod transports;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Executor};

use crate::config::ServerConfig;
use crate::schedule::{job_resource_need, pick_job, ResourceManager};
use crate::store::{
    now_rfc3339, ArtifactEntry, EventRecord, JobRecord, JobStatus, JobStore, NodeStatus, Priority,
    RestartPolicy, StoreError,
};

pub use transports::{TransportEndpoint, TransportStatus};

use observe::ObservingBinder;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("{0}")]
    Store(#[from] StoreError),
    #[error("{0}")]
    Other(String),
}

impl EngineError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Store(e) => e.exit_code(),
            Self::Other(_) => 1,
        }
    }
}

struct Inner {
    store: JobStore,
    resources: ResourceManager,
    cfg: ServerConfig,
    data_dir: PathBuf,
    running: HashSet<String>,
    /// job_id → leased resources
    leases: HashMap<String, BTreeMap<String, u32>>,
    stop: Arc<AtomicBool>,
    transports: TransportStatus,
}

#[derive(Clone)]
pub struct Engine {
    inner: Arc<Mutex<Inner>>,
}

impl Engine {
    pub fn start(data_dir: PathBuf, cfg: ServerConfig) -> Result<Self, EngineError> {
        fs::create_dir_all(&data_dir).map_err(|e| EngineError::Other(e.to_string()))?;
        let store = JobStore::open(&data_dir)?;
        let resources = ResourceManager::new(&cfg.resource_classes);
        let stop = Arc::new(AtomicBool::new(false));
        let engine = Self {
            inner: Arc::new(Mutex::new(Inner {
                store,
                resources,
                cfg: cfg.clone(),
                data_dir,
                running: HashSet::new(),
                leases: HashMap::new(),
                stop: Arc::clone(&stop),
                transports: TransportStatus::default(),
            })),
        };

        let tick = engine.clone();
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let _ = tick.tick();
                thread::sleep(Duration::from_millis(200));
            }
        });

        Ok(engine)
    }

    pub fn set_transports(&self, transports: TransportStatus) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.transports = transports;
        }
    }

    pub fn transports(&self) -> TransportStatus {
        self.inner
            .lock()
            .map(|i| i.transports.clone())
            .unwrap_or_default()
    }

    pub fn stop(&self) {
        if let Ok(inner) = self.inner.lock() {
            inner.stop.store(true, Ordering::SeqCst);
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.inner
            .lock()
            .map(|i| i.stop.load(Ordering::SeqCst))
            .unwrap_or(true)
    }

    pub fn data_dir(&self) -> PathBuf {
        self.inner
            .lock()
            .map(|i| i.data_dir.clone())
            .unwrap_or_default()
    }

    pub fn job_store(&self) -> Result<JobStore, EngineError> {
        self.inner
            .lock()
            .map(|i| i.store.clone())
            .map_err(|e| EngineError::Other(e.to_string()))
    }

    pub fn submit(
        &self,
        job: vd_pipeline::Job,
        priority: Priority,
        restart: RestartPolicy,
    ) -> Result<JobRecord, EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        let record = inner.store.create(job, priority, restart)?;
        Ok(record)
    }

    pub fn job(&self, id: &str) -> Result<JobRecord, EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        let mut rec = inner.store.load(id)?;
        merge_progress_snapshot(&inner.store.job_dir(id), &mut rec);
        Ok(rec)
    }

    pub fn list(&self) -> Result<Vec<JobRecord>, EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        let mut jobs = inner.store.list()?;
        for rec in &mut jobs {
            merge_progress_snapshot(&inner.store.job_dir(&rec.id), rec);
        }
        Ok(jobs)
    }

    pub fn events(&self, id: &str) -> Result<Vec<crate::store::EventRecord>, EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        Ok(inner.store.read_events(id)?)
    }

    pub fn append_event(&self, id: &str, event: EventRecord) -> Result<(), EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        Ok(inner.store.append_event(id, event)?)
    }

    /// Pick a Ready node matching step id (preferred) or capability; mark Running.
    pub fn mark_node_started(
        &self,
        job_id: &str,
        capability: &str,
        step_id: Option<&str>,
    ) -> Option<String> {
        let inner = self.inner.lock().ok()?;
        let mut rec = inner.store.load(job_id).ok()?;
        let node = rec.nodes.iter_mut().find(|n| {
            if !matches!(n.status, NodeStatus::Ready | NodeStatus::Pending) {
                return false;
            }
            if let Some(id) = step_id {
                n.id == id
            } else {
                n.capability == capability
            }
        })?;
        node.status = NodeStatus::Running;
        node.started_at = Some(now_rfc3339());
        let id = node.id.clone();
        let _ = inner.store.save(&rec);
        Some(id)
    }

    pub fn mark_node_finished(
        &self,
        job_id: &str,
        node_id: &str,
        status: NodeStatus,
        error: Option<String>,
    ) -> Result<(), EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        let mut rec = inner.store.load(job_id)?;
        if let Some(n) = rec.nodes.iter_mut().find(|n| n.id == node_id) {
            n.status = status;
            n.finished_at = Some(now_rfc3339());
            n.error = error;
        }
        inner.store.save(&rec)?;
        Ok(())
    }

    pub fn artifacts(&self, id: &str) -> Result<Vec<ArtifactEntry>, EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        Ok(inner.store.read_artifacts(id)?)
    }

    pub fn logs(&self, id: &str, stderr: bool) -> Result<String, EngineError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        Ok(inner.store.read_log(id, stderr)?)
    }

    pub fn cancel(&self, id: &str) -> Result<JobRecord, EngineError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        let mut rec = inner.store.load(id)?;
        if rec.status.is_terminal() {
            return Ok(rec);
        }
        rec.status = JobStatus::Cancelled;
        rec.finished_at = Some(now_rfc3339());
        for n in &mut rec.nodes {
            if !n.status.is_terminal() {
                n.status = NodeStatus::Cancelled;
            }
        }
        if let Some(lease) = inner.leases.remove(id) {
            inner.resources.release(&lease);
        }
        inner.running.remove(id);
        inner.store.save(&rec)?;
        inner.store.append_event(
            id,
            EventRecord {
                ts: now_rfc3339(),
                kind: "JobCancelled".into(),
                node_id: None,
                message: None,
                fields: Default::default(),
            },
        )?;
        Ok(rec)
    }

    pub fn resources_snapshot(&self) -> BTreeMap<String, (u32, u32)> {
        self.inner
            .lock()
            .map(|i| i.resources.snapshot())
            .unwrap_or_default()
    }

    pub fn workers_busy(&self) -> usize {
        self.inner.lock().map(|i| i.running.len()).unwrap_or(0)
    }

    pub fn workers_total(&self) -> u32 {
        self.inner.lock().map(|i| i.cfg.workers).unwrap_or(1)
    }

    /// Shared Operator health payload (all transports).
    pub fn health_json(&self) -> serde_json::Value {
        serde_json::json!({
            "workers_busy": self.workers_busy(),
            "workers": self.workers_total(),
            "resources": self.resources_snapshot(),
            "data_dir": self.data_dir(),
        })
    }

    fn tick(&self) -> Result<(), EngineError> {
        let (job_id, job, lease, job_dir) = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|e| EngineError::Other(e.to_string()))?;
            let workers = inner.cfg.workers as usize;
            let busy = inner.running.len();
            let jobs = inner.store.list()?;
            let Some(cand) = pick_job(&jobs, busy, workers).map(|j| j.id.clone()) else {
                return Ok(());
            };
            let mut rec = inner.store.load(&cand)?;
            let need = job_resource_need(&rec.nodes);
            if !inner.resources.lease(&need) {
                if rec.status != JobStatus::WaitingResources {
                    rec.status = JobStatus::WaitingResources;
                    inner.store.save(&rec)?;
                    inner.store.append_event(
                        &cand,
                        EventRecord {
                            ts: now_rfc3339(),
                            kind: "JobWaitingResources".into(),
                            node_id: None,
                            message: Some(format!("need {need:?}")),
                            fields: Default::default(),
                        },
                    )?;
                }
                return Ok(());
            }
            rec.status = JobStatus::Running;
            rec.started_at = Some(now_rfc3339());
            rec.progress = Some(0);
            rec.phase = Some("starting".into());
            // Leave nodes Ready — ObservingBinder advances them per leaf.
            for n in &mut rec.nodes {
                if matches!(
                    n.status,
                    NodeStatus::WaitingDependencies | NodeStatus::Pending
                ) {
                    n.status = NodeStatus::Ready;
                }
            }
            inner.store.save(&rec)?;
            inner.store.append_event(
                &cand,
                EventRecord {
                    ts: now_rfc3339(),
                    kind: "JobStarted".into(),
                    node_id: None,
                    message: None,
                    fields: Default::default(),
                },
            )?;
            inner.running.insert(cand.clone());
            inner.leases.insert(cand.clone(), need.clone());
            let job_dir = inner.store.job_dir(&cand);
            (cand, rec.job.clone(), need, job_dir)
        };

        let engine = self.clone();
        thread::spawn(move || {
            let result = run_job(&engine, &job_id, &job, &job_dir);
            let _ = engine.finish_job(&job_id, result, lease);
        });
        Ok(())
    }

    fn finish_job(
        &self,
        id: &str,
        result: Result<PathBuf, String>,
        lease: BTreeMap<String, u32>,
    ) -> Result<(), EngineError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| EngineError::Other(e.to_string()))?;
        let mut rec = inner.store.load(id)?;
        inner.resources.release(&lease);
        inner.leases.remove(id);
        inner.running.remove(id);

        let finished = now_rfc3339();
        rec.finished_at = Some(finished.clone());
        match result {
            Ok(out) => {
                rec.status = JobStatus::Completed;
                rec.exit_code = Some(0);
                rec.progress = Some(100);
                rec.phase = Some("done".into());
                for n in &mut rec.nodes {
                    if !n.status.is_terminal() || matches!(n.status, NodeStatus::Running) {
                        n.status = NodeStatus::Completed;
                        n.finished_at = Some(finished.clone());
                    }
                }
                let arts = vec![ArtifactEntry {
                    id: "primary".into(),
                    path: out.clone(),
                    kind: None,
                    producer: Some("executor".into()),
                }];
                inner.store.write_artifacts(id, &arts)?;
                let msg = Some(out.display().to_string());
                inner.store.append_event(
                    id,
                    EventRecord {
                        ts: finished.clone(),
                        kind: "JobFinished".into(),
                        node_id: None,
                        message: msg.clone(),
                        fields: Default::default(),
                    },
                )?;
                // ADR 0007 alias for HTTP/gRPC SSE clients.
                inner.store.append_event(
                    id,
                    EventRecord {
                        ts: finished,
                        kind: "JobCompleted".into(),
                        node_id: None,
                        message: msg,
                        fields: Default::default(),
                    },
                )?;
            }
            Err(err) => {
                rec.status = JobStatus::Failed;
                rec.exit_code = Some(1);
                rec.error = Some(err.clone());
                rec.phase = Some("error".into());
                for n in &mut rec.nodes {
                    if matches!(n.status, NodeStatus::Running | NodeStatus::Ready) {
                        n.status = NodeStatus::Failed;
                        n.finished_at = Some(finished.clone());
                        n.error = Some(err.clone());
                    }
                }
                let _ = append_stderr(&inner.store.job_dir(id), &err);
                inner.store.append_event(
                    id,
                    EventRecord {
                        ts: finished,
                        kind: "JobFailed".into(),
                        node_id: None,
                        message: Some(err),
                        fields: Default::default(),
                    },
                )?;
            }
        }
        inner.store.save(&rec)?;
        Ok(())
    }
}

fn run_job(
    engine: &Engine,
    job_id: &str,
    job: &vd_pipeline::Job,
    job_dir: &Path,
) -> Result<PathBuf, String> {
    let resolved = resolve_job(job.clone()).map_err(|e| e.to_string())?;
    let binder = ObservingBinder::new(engine.clone(), job_id.to_string());
    let executor = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: Some(job_dir.join("progress.json")),
    };
    match executor.run(&resolved) {
        Ok(out) => {
            let _ = write_metrics(job_dir, &out.report);
            Ok(out.output)
        }
        Err(fail) => {
            let _ = write_metrics(job_dir, &fail.report);
            Err(fail.to_string())
        }
    }
}

/// Overlay live Executor snapshot onto the Job record for `job.status` / `get_job`.
fn merge_progress_snapshot(job_dir: &Path, rec: &mut JobRecord) {
    if matches!(
        rec.status,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    ) {
        if rec.progress.is_none() && matches!(rec.status, JobStatus::Completed) {
            rec.progress = Some(100);
        }
        return;
    }
    let path = job_dir.join("progress.json");
    let Ok(body) = fs::read_to_string(&path) else {
        return;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
        return;
    };
    if let Some(p) = v.get("percent").and_then(|x| x.as_u64()) {
        rec.progress = Some(p.min(100) as u8);
    }
    if let Some(ph) = v.get("phase").and_then(|x| x.as_str()) {
        rec.phase = Some(ph.to_string());
    }
}

fn write_metrics(job_dir: &Path, report: &vd_pipeline::ExecutionReport) -> Result<(), String> {
    let body = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    fs::write(job_dir.join("metrics.json"), body).map_err(|e| e.to_string())?;
    let timeline: Vec<_> = report
        .steps
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "capability": s.capability,
                "status": s.status,
                "duration_ms": s.duration_ms,
            })
        })
        .collect();
    let body = serde_json::to_string_pretty(&timeline).map_err(|e| e.to_string())?;
    fs::write(job_dir.join("timeline.json"), body).map_err(|e| e.to_string())
}

fn append_stderr(job_dir: &Path, msg: &str) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(job_dir.join("stderr.log"))
        .map_err(|e| e.to_string())?;
    writeln!(f, "{msg}").map_err(|e| e.to_string())
}
