//! Executor builds ExecutionReport with per-step timings.

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{
    resolve_job, ArgValue, Capability, Executor, Job, JobInput, JobReportStatus, Step,
    StepReportStatus,
};

use super::RecordingBinder;

#[test]
fn report_records_step_order_and_timings() {
    let binder = RecordingBinder::new();
    let mut options = std::collections::BTreeMap::new();
    options.insert("engine".into(), ArgValue::String("gigaam".into()));
    options.insert("model".into(), ArgValue::String("v3_e2e_ctc".into()));

    let job = Job {
        version: 1,
        name: Some("timing".into()),
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("a.ogg")),
        },
        context: Default::default(),
        output: Default::default(),
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![
            Step {
                id: Some("transcript".into()),
                output: Some(PathBuf::from("/work/t.txt")),
                options: options.clone(),
                ..Step::new(Capability::Transcribe)
            },
            Step {
                input: Some("transcript".into()),
                output: Some(PathBuf::from("/work/c.txt")),
                ..Step::new(Capability::FixCasing)
            },
        ].into_iter().map(Into::into).collect(),
    };
    let resolved = resolve_job(job).unwrap();
    // Small sleep so wall clock advances between steps (stub is instant).
    thread::sleep(Duration::from_millis(5));
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    let out = exec.run(&resolved).unwrap();
    assert_eq!(out.report.status, JobReportStatus::Ok);
    assert_eq!(out.report.job.as_deref(), Some("timing"));
    assert_eq!(out.report.steps.len(), 2);
    assert_eq!(out.report.steps[0].id, "transcript");
    assert_eq!(out.report.steps[0].capability, "transcribe");
    assert_eq!(out.report.steps[0].status, StepReportStatus::Ok);
    assert_eq!(out.report.steps[0].backend.as_deref(), Some("gigaam"));
    assert_eq!(out.report.steps[0].model.as_deref(), Some("v3_e2e_ctc"));
    assert_eq!(out.report.steps[1].capability, "fix-casing");
    assert!(out.report.duration_ms >= out.report.steps[0].duration_ms);
    assert!(!out.report.started_at.is_empty());
    assert!(!out.report.finished_at.is_empty());
}

#[test]
fn report_on_failure_is_partial() {
    let binder = RecordingBinder::failing(Capability::FixCasing);
    let job = Job {
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
        continue_on_error: false,
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
        ].into_iter().map(Into::into).collect(),
    };
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    let err = exec.run(&resolved).unwrap_err();
    assert_eq!(err.report.status, JobReportStatus::Failed);
    assert_eq!(err.report.steps.len(), 2);
    assert_eq!(err.report.steps[0].status, StepReportStatus::Ok);
    assert_eq!(err.report.steps[1].status, StepReportStatus::Failed);
}
