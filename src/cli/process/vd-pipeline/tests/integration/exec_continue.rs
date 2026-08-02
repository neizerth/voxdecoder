//! continue_on_error behavior.

use std::path::PathBuf;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Capability, Executor, Job, JobInput, Step};

use super::RecordingBinder;

fn job(continue_on_error: bool) -> Job {
    Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("a.ogg")),
        },
        context: Default::default(),
        output: Default::default(),
        max_parallel: None,
        resources: Default::default(),
        continue_on_error,
        steps: vec![
            Step {
                id: Some("transcript".into()),
                output: Some(PathBuf::from("/work/t.txt")),
                ..Step::new(Capability::Transcribe)
            },
            Step {
                input: Some("transcript".into()),
                output: Some(PathBuf::from("/work/c.txt")),
                ..Step::new(Capability::FixCasing)
            },
            Step {
                output: Some(PathBuf::from("/work/a.txt")),
                ..Step::new(Capability::FixAsr)
            },
        ].into_iter().map(Into::into).collect(),
    }
}

#[test]
fn failure_stops_by_default() {
    let binder = RecordingBinder::failing(Capability::FixCasing);
    let resolved = resolve_job(job(false)).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    let fail = exec.run(&resolved).unwrap_err();
    assert_eq!(binder.calls.lock().unwrap().len(), 2);
    assert_eq!(fail.report.status, vd_pipeline::JobReportStatus::Failed);
    assert_eq!(
        fail.report.steps.last().unwrap().status,
        vd_pipeline::StepReportStatus::Failed
    );
}

#[test]
fn continue_on_error_runs_rest() {
    let binder = RecordingBinder::failing(Capability::FixCasing);
    let resolved = resolve_job(job(true)).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    let out = exec.run(&resolved).unwrap();
    assert_eq!(out.output, PathBuf::from("/work/a.txt"));
    assert_eq!(binder.calls.lock().unwrap().len(), 3);
    assert_eq!(out.report.status, vd_pipeline::JobReportStatus::Failed);
    assert_eq!(out.report.steps[1].status, vd_pipeline::StepReportStatus::Failed);
    assert_eq!(out.report.steps[2].status, vd_pipeline::StepReportStatus::Ok);
}
