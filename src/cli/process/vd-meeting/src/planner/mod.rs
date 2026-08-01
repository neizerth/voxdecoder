//! MeetingPlanner — validate · normalize · plan Job · submit.

mod artifacts;
mod graph;
mod normalize;
mod submit;

use vd_pipeline::Job;

use crate::model::{BuildOptions, MeetingRequest};

pub use normalize::{require_paths, ResolvedMeeting};
pub use submit::submit_job;

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Other(String),
}

impl PlanError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::NotFound(_) => 3,
            Self::Other(_) => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlannedJob {
    pub job: Job,
    pub resolved: ResolvedMeeting,
}

pub struct MeetingPlanner;

impl MeetingPlanner {
    pub fn plan(
        request: &MeetingRequest,
        options: &BuildOptions,
    ) -> Result<PlannedJob, PlanError> {
        let resolved = normalize::normalize(request)?;
        let job = graph::build_job(&resolved, options)?;
        Ok(PlannedJob { job, resolved })
    }
}

/// Plan MeetingRequest + BuildOptions → Job.
pub fn plan_job(
    request: &MeetingRequest,
    options: &BuildOptions,
) -> Result<Job, PlanError> {
    Ok(MeetingPlanner::plan(request, options)?.job)
}
