//! `vd-preprocess` — media filter-chain executor (CLI ≡ `use: preprocess`).

pub mod cli;
pub mod config;
pub mod paths;
pub mod preprocess;
pub mod status;

pub use preprocess::{
    execute, plan, request_from_raw, ExecutionPlan, FilterSpec, MediaProviderSpec, PlannedFilter,
    PreparedMedia, PreprocessError, PreprocessRequest, PreprocessResult, RawFilter,
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
