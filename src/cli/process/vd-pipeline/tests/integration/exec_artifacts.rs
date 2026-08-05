//! Artifact id → next step input.

use std::path::PathBuf;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Capability, Executor, Job, JobInput, Step, WorkflowNode};

use super::RecordingBinder;

#[test]
fn artifact_id_wires_path() {
    let binder = RecordingBinder::new();
    let job = Job {
        version: 1,
        id: None,
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
                output: Some(PathBuf::from("/work/transcript.txt")),
                ..Step::new(Capability::Transcribe)
            },
            Step {
                input: Some("transcript".into()),
                output: Some(PathBuf::from("/work/fixed.txt")),
                ..Step::new(Capability::FixCasing)
            },
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
    };
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    exec.run(&resolved).unwrap();
    let calls = binder.calls.lock().unwrap();
    assert_eq!(calls[0].input, PathBuf::from("/work/a.ogg"));
    assert_eq!(calls[1].input, PathBuf::from("/work/transcript.txt"));
}

#[test]
fn dotted_meeting_ids_wire_across_parallel_branches() {
    let binder = RecordingBinder::new();
    let job = Job {
        version: 1,
        id: None,
        name: Some("meeting".into()),
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("igor.wav")),
        },
        context: Default::default(),
        output: Default::default(),
        max_parallel: Some(2),
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![WorkflowNode::parallel(vec![
            WorkflowNode::sequence(vec![
                Step {
                    id: Some("igor.transcript".into()),
                    input: Some("igor.wav".into()),
                    output: Some(PathBuf::from("/work/igor.txt")),
                    ..Step::new(Capability::Transcribe)
                }
                .into(),
                Step {
                    id: Some("igor.cased".into()),
                    inputs: vec!["igor.transcript".into()],
                    output: Some(PathBuf::from("/work/igor.cased.txt")),
                    ..Step::new(Capability::FixCasing)
                }
                .into(),
            ]),
            WorkflowNode::sequence(vec![
                Step {
                    id: Some("vladimir.transcript".into()),
                    input: Some("vladimir.wav".into()),
                    output: Some(PathBuf::from("/work/vladimir.txt")),
                    ..Step::new(Capability::Transcribe)
                }
                .into(),
                Step {
                    id: Some("vladimir.cased".into()),
                    inputs: vec!["vladimir.transcript".into()],
                    output: Some(PathBuf::from("/work/vladimir.cased.txt")),
                    ..Step::new(Capability::FixCasing)
                }
                .into(),
            ]),
        ])],
    };
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    exec.run(&resolved).unwrap();
    let calls = binder.calls.lock().unwrap();
    assert_eq!(calls.len(), 4);

    let igor_cased = calls
        .iter()
        .find(|c| c.input == PathBuf::from("/work/igor.txt"))
        .expect("igor.cased must read igor.transcript artifact path");
    assert_eq!(igor_cased.capability, Capability::FixCasing);

    let vladimir_cased = calls
        .iter()
        .find(|c| c.input == PathBuf::from("/work/vladimir.txt"))
        .expect("vladimir.cased must read vladimir.transcript artifact path");
    assert_eq!(vladimir_cased.capability, Capability::FixCasing);

    // Must not pass literal "{name}.transcript" paths (old ArtifactRef bug).
    assert!(!calls
        .iter()
        .any(|c| c.input.ends_with("igor.transcript") || c.input.ends_with("vladimir.transcript")));
}
