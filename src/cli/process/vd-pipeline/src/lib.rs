//! `vd-pipeline` — build and execute a VoxDecoder Job.

pub mod artifacts;
pub mod cli;
pub mod config;
pub mod exec;
pub mod interactive;
pub mod job;
pub mod meeting_artifact;
pub mod paths;
pub mod report;
pub mod status;

pub use exec::{
    Binder, ExecError, ExecFailure, ExecOutcome, Executor, InvokeRequest, InvokeResult,
    SubprocessBinder,
};
pub use job::{
    default_job, default_preprocess_filters, is_video_path, load_job_file, resolve_job, ArgValue,
    ArtifactRef, Capability, DefaultJobArgs, Job, JobContext, JobError, JobInput, JobOutput,
    ResolvedJob, ResolvedStep, Step, TranscribeEngine, WorkflowNode, WorkflowPlan,
};
pub use interactive::{MenuItem, MenuOutcome};
pub use meeting_artifact::{
    MeetingArtifact, MeetingTurn, MergeStrategy, SpeakerSegment, SpeakerTimeline,
};
pub use report::{
    backend_from_options, format_rfc3339, model_from_options, ArtifactStat, ExecutionReport,
    JobReportStatus, PhaseReport, StepReport, StepReportStatus,
};

pub use vd_artifact as artifact;
pub use vd_progress as progress;

use std::ffi::OsString;
use std::process::ExitCode;

use cli::parse_args;

/// Parse argv and dispatch. Returns a process exit code.
pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match parse_args(args) {
        Ok(cmd) => match cli::dispatch(cmd) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                if !err.message().is_empty() {
                    eprintln!("{err}");
                }
                ExitCode::from(err.exit_code())
            }
        },
        Err(err) => {
            if !err.message().is_empty() {
                eprintln!("{err}");
            }
            ExitCode::from(err.exit_code())
        }
    }
}
