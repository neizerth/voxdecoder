//! Step order and skip.

use std::path::PathBuf;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Capability, Executor, Job, JobInput, Step};

use super::RecordingBinder;

fn base_job(steps: Vec<Step>) -> Job {
    Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("a.ogg")),
        },
        context: Default::default(),
        output: Default::default(),
        continue_on_error: false,
        steps,
    }
}

#[test]
fn steps_run_in_order() {
    let binder = RecordingBinder::new();
    let job = base_job(vec![
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
    ]);
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    exec.run(&resolved).unwrap();
    let caps: Vec<_> = binder.calls.borrow().iter().map(|c| c.capability).collect();
    assert_eq!(
        caps,
        vec![
            Capability::Transcribe,
            Capability::FixCasing,
            Capability::FixAsr
        ]
    );
}

#[test]
fn skip_does_not_invoke() {
    let binder = RecordingBinder::new();
    let job = base_job(vec![
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
            output: None,
            skip: true,
            options: Default::default(),
        },
        Step {
            r#use: Capability::FixAsr,
            id: None,
            name: None,
            input: Some("transcript".into()),
            output: Some(PathBuf::from("/work/a.txt")),
            skip: false,
            options: Default::default(),
        },
    ]);
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    exec.run(&resolved).unwrap();
    let caps: Vec<_> = binder.calls.borrow().iter().map(|c| c.capability).collect();
    assert_eq!(caps, vec![Capability::Transcribe, Capability::FixAsr]);
}
