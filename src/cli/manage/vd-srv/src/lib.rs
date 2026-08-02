//! `vd-srv` — VoxDecoder execution engine.

#![allow(
    clippy::large_enum_variant,
    clippy::significant_drop_tightening,
    clippy::assigning_clones,
    clippy::needless_lifetimes,
    clippy::derivable_impls,
    clippy::unused_self,
    clippy::match_same_arms,
    clippy::ref_option,
    clippy::default_trait_access,
    clippy::redundant_clone
)]

pub mod api;
pub mod cli;
pub mod config;
pub mod engine;
pub mod paths;
pub mod schedule;
pub mod store;

pub use engine::{Engine, EngineError};
pub use schedule::{pick_job, ResourceManager};
pub use store::{
    JobRecord, JobStatus, JobStore, NodeStatus, Priority, RestartPolicy, StoreError,
};

use std::ffi::OsString;
use std::process::ExitCode;

use cli::parse_args;

/// Parse argv and dispatch.
pub fn run_cli<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match parse_args(args) {
        Ok(cmd) => match cli::dispatch(cmd) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                if err.exit_code() == 0 {
                    print!("{}", err.message());
                    return ExitCode::SUCCESS;
                }
                if !err.message().is_empty() {
                    eprintln!("{err}");
                }
                ExitCode::from(err.exit_code())
            }
        },
        Err(err) => {
            if err.exit_code() == 0 {
                print!("{}", err.message());
                return ExitCode::SUCCESS;
            }
            if !err.message().is_empty() {
                eprintln!("{err}");
            }
            ExitCode::from(err.exit_code())
        }
    }
}
