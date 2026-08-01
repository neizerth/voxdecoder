//! Job schema, parse, default builder, resolve.

mod default;
pub(crate) mod parse;
pub(crate) mod resolve;
mod schema;

pub use default::{default_job, DefaultJobArgs};
pub use parse::load_job_file;
pub use resolve::resolve_job;
pub use schema::{
    ArgValue, ArtifactRef, Capability, Job, JobContext, JobError, JobInput, JobOutput, ResolvedJob,
    ResolvedStep, Step, TranscribeEngine,
};
