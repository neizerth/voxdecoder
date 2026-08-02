//! prepare-context is always in the default Job.

use std::path::PathBuf;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{
    default_job, resolve_job, Capability, DefaultJobArgs, Executor, TranscribeEngine,
};

use super::RecordingBinder;

#[test]
fn default_without_docs_still_invokes_prepare_context() {
    let binder = RecordingBinder::new();
    let mut job = default_job(&DefaultJobArgs {
        audio: PathBuf::from("/work/a.ogg"),
        engine: TranscribeEngine::Gigaam,
        model: None,
        device: None,
        flash: false,
        speed: None,
        docs: None,
        output_dir: None,
        working_dir: Some(PathBuf::from("/work")),
        continue_on_error: false,
        overwrite: false,
    });
    for (i, step) in job.steps.iter_mut().enumerate() {
        if let vd_pipeline::WorkflowNode::Step(s) = step {
            s.output = Some(PathBuf::from(format!("/work/out-{i}.txt")));
        }
    }
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    exec.run(&resolved).unwrap();
    assert!(binder
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|c| c.capability == Capability::PrepareContext));
}

#[test]
fn default_with_docs_invokes_prepare_context() {
    let binder = RecordingBinder::new();
    let mut job = default_job(&DefaultJobArgs {
        audio: PathBuf::from("/work/a.ogg"),
        engine: TranscribeEngine::Gigaam,
        model: None,
        device: None,
        flash: false,
        speed: None,
        docs: Some(PathBuf::from("docs")),
        output_dir: None,
        working_dir: Some(PathBuf::from("/work")),
        continue_on_error: false,
        overwrite: false,
    });
    for (i, step) in job.steps.iter_mut().enumerate() {
        if let vd_pipeline::WorkflowNode::Step(s) = step {
            s.output = Some(PathBuf::from(format!("/work/out-{i}.txt")));
        }
    }
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    exec.run(&resolved).unwrap();
    assert!(binder
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|c| c.capability == Capability::PrepareContext));
}
