//! `vdctl` — Platform CLI for the local VoxDecoder installation.

#![allow(clippy::module_name_repetitions)]

pub mod agents;
pub mod assets;
pub mod client;
pub mod cli;
pub mod config;
pub mod discover;
pub mod doctor;
pub mod error;
pub mod lifecycle;
pub mod link;
pub mod mcp;
pub mod output;
pub mod paths;
pub mod resolve;
pub mod skills;
pub mod update;

use std::ffi::OsString;
use std::process::ExitCode;

/// Parse argv and dispatch. Returns a process exit code.
pub fn run_cli<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match cli::run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !error.message().is_empty() {
                eprintln!("{error}");
            }
            ExitCode::from(error.exit_code())
        }
    }
}
