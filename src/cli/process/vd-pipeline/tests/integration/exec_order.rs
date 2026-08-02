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
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: steps.into_iter().map(Into::into).collect(),
    }
}

#[test]
fn steps_run_in_order() {
    let binder = RecordingBinder::new();
    let job = base_job(vec![
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
    ]);
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    exec.run(&resolved).unwrap();
    let caps: Vec<_> = binder.calls.lock().unwrap().iter().map(|c| c.capability).collect();
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
            id: Some("transcript".into()),
            output: Some(PathBuf::from("/work/t.txt")),
            ..Step::new(Capability::Transcribe)
        },
        Step {
            input: Some("transcript".into()),
            skip: true,
            ..Step::new(Capability::FixCasing)
        },
        Step {
            input: Some("transcript".into()),
            output: Some(PathBuf::from("/work/a.txt")),
            ..Step::new(Capability::FixAsr)
        },
    ]);
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    exec.run(&resolved).unwrap();
    let caps: Vec<_> = binder.calls.lock().unwrap().iter().map(|c| c.capability).collect();
    assert_eq!(caps, vec![Capability::Transcribe, Capability::FixAsr]);
}
