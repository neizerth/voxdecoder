//! Node / Job scheduler (v1: Job-granularity dispatch).

mod resources;

pub use resources::{job_resource_need, ResourceManager};

use crate::store::{JobRecord, JobStatus, Priority};

/// Pick the next queued Job to run (highest priority, then oldest).
pub fn pick_job<'a>(jobs: &'a [JobRecord], busy: usize, workers: usize) -> Option<&'a JobRecord> {
    if busy >= workers {
        return None;
    }
    let mut candidates: Vec<&JobRecord> = jobs
        .iter()
        .filter(|j| matches!(j.status, JobStatus::Queued | JobStatus::WaitingResources))
        .collect();
    candidates.sort_by(|a, b| {
        b.priority
            .rank()
            .cmp(&a.priority.rank())
            .then_with(|| a.created_at.cmp(&b.created_at))
    });
    candidates.first().copied()
}

pub fn priority_label(p: Priority) -> &'static str {
    p.as_str()
}
