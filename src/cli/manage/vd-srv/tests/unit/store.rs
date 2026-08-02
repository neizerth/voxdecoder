//! JobStore round-trip.

use std::path::PathBuf;

use tempfile::TempDir;
use vd_pipeline::{Capability, Job, JobInput, Step};
use vd_srv::{JobStatus, JobStore, Priority, RestartPolicy};

fn sample_job() -> Job {
    Job {
        version: 1,
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
