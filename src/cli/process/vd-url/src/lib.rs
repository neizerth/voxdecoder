//! `vd-url` — online media importer (CLI ≡ `use: import-url`).
//!
//! One library for the CLI and the Runtime capability.

pub mod artifact;
pub mod cli;
pub mod config;
pub mod import;
pub mod paths;
pub mod provider;

pub use import::{
    detect_provider, parse_url_ok, resolve, validate_request, ImportError, ImportResult, ProviderId,
    SubtitlePolicy, UrlImportRequest,
};
pub use provider::{catalog_lines, doctor_report, MediaProvider};

pub use vd_artifact as artifact_crate;
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
