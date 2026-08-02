//! Contended accelerator resources serialize leaf invokes across parallel branches.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{
    resolve_job, Binder, Capability, ExecError, Executor, InvokeRequest, InvokeResult, Job,
    JobInput, Step, WorkflowNode,
};

use super::RecordingBinder;

/// Binder that holds the invoke for a bit so concurrent leases are observable.
struct SlowGpuBinder {
    inner: RecordingBinder,
    concurrent: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl Binder for SlowGpuBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        let now = self.concurrent.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(40));
        let out = self.inner.invoke(req);
        self.concurrent.fetch_sub(1, Ordering::SeqCst);
        out
    }
}

impl Binder for &SlowGpuBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        (*self).invoke(req)
    }
}

#[test]
fn parallel_transcribe_branches_share_one_metal_slot() {
    let concurrent = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let binder = SlowGpuBinder {
        inner: RecordingBinder::new(),
        concurrent: Arc::clone(&concurrent),
        peak: Arc::clone(&peak),
    };

    let job = Job {
        version: 1,
        name: Some("metal-gate".into()),
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("a.ogg")),
        },
        context: Default::default(),
        output: Default::default(),
        max_parallel: Some(4),
        resources: std::collections::BTreeMap::from([("metal_gpu".into(), 1)]),
        continue_on_error: false,
        steps: vec![WorkflowNode::parallel(vec![
            Step {
                id: Some("a".into()),
                input: Some("/work/a.ogg".into()),
                output: Some(PathBuf::from("/work/a.txt")),
                resource: Some("metal_gpu".into()),
                ..Step::new(Capability::Transcribe)
            }
            .into(),
            Step {
                id: Some("b".into()),
                input: Some("/work/b.ogg".into()),
                output: Some(PathBuf::from("/work/b.txt")),
                resource: Some("metal_gpu".into()),
                ..Step::new(Capability::Transcribe)
            }
            .into(),
            Step {
                id: Some("c".into()),
                input: Some("/work/c.ogg".into()),
                output: Some(PathBuf::from("/work/c.txt")),
                resource: Some("metal_gpu".into()),
                ..Step::new(Capability::Transcribe)
            }
            .into(),
        ])],
    };
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    let t0 = Instant::now();
    exec.run(&resolved).unwrap();
    let elapsed = t0.elapsed();

    assert_eq!(
        peak.load(Ordering::SeqCst),
        1,
        "metal_gpu capacity 1 must serialize concurrent Transcribe invokes"
    );
    // Three serial 40ms holds ⇒ well above one parallel wave.
    assert!(
        elapsed >= Duration::from_millis(100),
        "expected serialized runtime, got {elapsed:?}"
    );
    assert_eq!(binder.inner.calls.lock().unwrap().len(), 3);
}
