//! Artifact id → next step input.

use std::path::PathBuf;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Capability, Executor, Job, JobInput, Step};

use super::RecordingBinder;

#[test]
fn artifact_id_wires_path() {
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
        continue_on_error: false,
        steps: vec![
            Step {
                r#use: Capability::Transcribe,
                id: Some("transcript".into()),
                name: None,
                input: None,
                output: Some(PathBuf::from("/work/transcript.txt")),
                skip: false,
                options: Default::default(),
            },
            Step {
                r#use: Capability::FixCasing,
                id: None,
                name: None,
                input: Some("transcript".into()),
                output: Some(PathBuf::from("/work/fixed.txt")),
                skip: false,
                options: Default::default(),
            },
        ],
    };
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    exec.run(&resolved).unwrap();
    let calls = binder.calls.borrow();
    assert_eq!(calls[0].input, PathBuf::from("/work/a.ogg"));
    assert_eq!(calls[1].input, PathBuf::from("/work/transcript.txt"));
}
