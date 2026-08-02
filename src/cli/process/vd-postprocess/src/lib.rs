//! `vd-postprocess` — recipe executor (CLI ≡ `use: postprocess`).

pub mod cli;
pub mod config;
pub mod paths;
pub mod postprocess;
pub mod status;

pub use postprocess::{
    execute, execute_with_progress, plan, ArtifactBinding, ArtifactOutput, DerivedArtifact,
    ExecutionNode, ExecutionPlan, ExecutionProviderSpec, ExecutionRunner, PlannedRecipe,
    PostprocessError, PostprocessRequest, PostprocessResult, RecipeDoc, RecipeResult, RunnerSpec,
};

pub use vd_artifact as artifact;
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
