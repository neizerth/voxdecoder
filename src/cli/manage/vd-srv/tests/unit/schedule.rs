//! Scheduler pick order.

use std::path::PathBuf;

use vd_pipeline::{Capability, Job, JobInput, Step};
use vd_srv::store::{JobRecord, JobStatus, Priority, RestartPolicy};
use vd_srv::pick_job;

fn rec(id: &str, status: JobStatus, priority: Priority) -> JobRecord {
    JobRecord {
        id: id.into(),
        status,
        priority,
        restart: RestartPolicy::Resume,
        created_at: id.into(),
        queued_at: None,
        started_at: None,
        finished_at: None,
        exit_code: None,
        error: None,
        job: Job {
            version: 1,
            name: None,
            working_dir: Some(PathBuf::from(".")),
            input: JobInput::default(),
            context: Default::default(),
            output: Default::default(),
            continue_on_error: false,
            max_parallel: None,
            resources: Default::default(),
            steps: vec![Step::new(Capability::FixCasing).into()],
        },
        nodes: vec![],
        working_dir: PathBuf::from("."),
    }
}

#[test]
fn prefers_high_priority() {
    let jobs = vec![
        rec("a", JobStatus::Queued, Priority::Low),
        rec("b", JobStatus::Queued, Priority::High),
        rec("c", JobStatus::Queued, Priority::Normal),
    ];
    let picked = pick_job(&jobs, 0, 1).unwrap();
    assert_eq!(picked.id, "b");
}

#[test]
fn respects_worker_cap() {
    let jobs = vec![rec("a", JobStatus::Queued, Priority::Normal)];
    assert!(pick_job(&jobs, 1, 1).is_none());
}
