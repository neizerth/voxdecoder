//! JobStore round-trip.

use std::path::PathBuf;

use tempfile::TempDir;
use vd_pipeline::{Capability, Job, JobInput, Step, WorkflowNode};
use vd_srv::{JobStatus, JobStore, Priority, RestartPolicy};

fn sample_job() -> Job {
    Job {
        version: 1,
        id: None,
        name: Some("t".into()),
        working_dir: Some(PathBuf::from(".")),
        input: JobInput::default(),
        context: Default::default(),
        output: Default::default(),
        continue_on_error: false,
        max_parallel: Some(1),
        resources: Default::default(),
        steps: vec![Step::new(Capability::FixCasing).into()],
    }
}

#[test]
fn create_and_load() {
    let dir = TempDir::new().unwrap();
    let store = JobStore::open(dir.path()).unwrap();
    let rec = store
        .create(sample_job(), Priority::Normal, RestartPolicy::Resume)
        .unwrap();
    assert_eq!(rec.status, JobStatus::Queued);
    assert!(!rec.nodes.is_empty());
    let loaded = store.load(&rec.id).unwrap();
    assert_eq!(loaded.id, rec.id);
    let events = store.read_events(&rec.id).unwrap();
    assert!(events.iter().any(|e| e.kind == "JobQueued"));
}

#[test]
fn parallel_meeting_branches_do_not_chain_depends_on() {
    let dir = TempDir::new().unwrap();
    let store = JobStore::open(dir.path()).unwrap();
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
        continue_on_error: false,
        max_parallel: Some(2),
        resources: Default::default(),
        steps: vec![WorkflowNode::parallel(vec![
            WorkflowNode::sequence(vec![
                Step {
                    id: Some("igor.transcript".into()),
                    input: Some("igor.wav".into()),
                    ..Step::new(Capability::Transcribe)
                }
                .into(),
                Step {
                    id: Some("igor.cased".into()),
                    inputs: vec!["igor.transcript".into()],
                    ..Step::new(Capability::FixCasing)
                }
                .into(),
                Step {
                    id: Some("igor.text".into()),
                    inputs: vec!["igor.cased".into()],
                    ..Step::new(Capability::FixTerms)
                }
                .into(),
            ]),
            WorkflowNode::sequence(vec![
                Step {
                    id: Some("vladimir.transcript".into()),
                    input: Some("vladimir.wav".into()),
                    ..Step::new(Capability::Transcribe)
                }
                .into(),
                Step {
                    id: Some("vladimir.cased".into()),
                    inputs: vec!["vladimir.transcript".into()],
                    ..Step::new(Capability::FixCasing)
                }
                .into(),
                Step {
                    id: Some("vladimir.text".into()),
                    inputs: vec!["vladimir.cased".into()],
                    ..Step::new(Capability::FixTerms)
                }
                .into(),
            ]),
        ])],
    };
    let rec = store
        .create(job, Priority::Normal, RestartPolicy::Resume)
        .unwrap();

    let vladimir_transcript = rec
        .nodes
        .iter()
        .find(|n| n.id == "vladimir.transcript")
        .expect("vladimir.transcript node");
    assert!(
        vladimir_transcript.depends_on.is_empty(),
        "second branch must not wait on first branch leaf (got {:?})",
        vladimir_transcript.depends_on
    );

    let igor_cased = rec
        .nodes
        .iter()
        .find(|n| n.id == "igor.cased")
        .expect("igor.cased node");
    assert_eq!(igor_cased.depends_on, vec!["igor.transcript".to_string()]);

    let vladimir_cased = rec
        .nodes
        .iter()
        .find(|n| n.id == "vladimir.cased")
        .expect("vladimir.cased node");
    assert_eq!(
        vladimir_cased.depends_on,
        vec!["vladimir.transcript".to_string()]
    );
}
