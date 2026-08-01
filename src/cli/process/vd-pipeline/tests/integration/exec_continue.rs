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
        continue_on_error,
        steps: vec![
            Step {
                r#use: Capability::Transcribe,
                id: Some("transcript".into()),
                name: None,
                input: None,
                output: Some(PathBuf::from("/work/t.txt")),
                skip: false,
                options: Default::default(),
            },
            Step {
                r#use: Capability::FixCasing,
                id: None,
                name: None,
                input: Some("transcript".into()),
                output: Some(PathBuf::from("/work/c.txt")),
                skip: false,
                options: Default::default(),
            },
            Step {
                r#use: Capability::FixAsr,
                id: None,
                name: None,
                input: None,
                output: Some(PathBuf::from("/work/a.txt")),
                skip: false,
                options: Default::default(),
            },
        ],
    }
}

#[test]
fn failure_stops_by_default() {
    let binder = RecordingBinder::failing(Capability::FixCasing);
    let resolved = resolve_job(job(false)).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    assert!(exec.run(&resolved).is_err());
    assert_eq!(binder.calls.borrow().len(), 2);
}

#[test]
fn continue_on_error_runs_rest() {
    let binder = RecordingBinder::failing(Capability::FixCasing);
    let resolved = resolve_job(job(true)).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    // prev stays at transcript; fix-asr gets input None → prev still transcript
    let out = exec.run(&resolved).unwrap();
    assert_eq!(out, PathBuf::from("/work/a.txt"));
    assert_eq!(binder.calls.borrow().len(), 3);
}
