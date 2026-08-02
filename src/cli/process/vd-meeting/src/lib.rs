//! `vd-meeting` — Meeting Planner (MeetingRequest + BuildOptions → Job).

pub mod cli;
pub mod config;
pub mod model;
pub mod paths;
pub mod planner;
pub mod status;

pub use model::{
    AlignmentMode, AlignmentOptions, BuildOptions, CountBounds, DiarizationEnabled,
    DiarizationPolicy, ExecutorOptions, Gender, GroupConstraints, InputPurpose, InputRole,
    InputSource, KnownParticipant, MeetingModel, MeetingOutput, MeetingRequest,
    ParticipantConstraints, Participants, TranscribeDefaults,
};
pub use planner::{plan_job, MeetingPlanner, PlanError, PlannedJob};

pub use vd_artifact as artifact;
pub use vd_pipeline as pipeline;
pub use vd_progress as progress;

use std::ffi::OsString;
use std::process::ExitCode;

use cli::parse_args;

/// Parse argv and dispatch. Returns a process exit code.
pub fn run_cli<I, T>(args: I) -> ExitCode
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
