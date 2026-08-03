//! `vd-fix-overlap` library: CLI, config, and duplicate-speech detection.
//!
//! Reads real JSON/JSONL diarized artifacts via `vd_artifact::collect_segments`
//! and, with `--apply`, removes duplicates via `vd_artifact::remove_segments` —
//! see `overlap::detect` for the pure detection function.

pub mod cli;
pub mod config;
pub mod overlap;
pub mod paths;
pub mod types;

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
