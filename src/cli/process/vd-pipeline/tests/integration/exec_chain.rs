//! Omitted input → previous primary output.

use std::path::PathBuf;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Capability, Executor, Job, JobInput, Step};

use super::RecordingBinder;

#[test]
fn omitted_input_uses_previous_output() {
    let binder = RecordingBinder::new();
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
            Step {
                output: Some(PathBuf::from("/work/a.txt")),
                ..Step::new(Capability::FixAsr)
            },
        ].into_iter().map(Into::into).collect(),
    };
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    exec.run(&resolved).unwrap();
    let calls = binder.calls.lock().unwrap();
    assert_eq!(calls[2].input, PathBuf::from("/work/c.txt"));
}
