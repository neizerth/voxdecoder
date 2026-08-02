//! `vd-mcp` — MCP gateway for the VoxDecoder Runtime API.

#![allow(clippy::large_enum_variant, clippy::unused_self)]

pub mod cli;
pub mod client;
pub mod config;
pub mod error;
pub mod mcp;
pub mod paths;
pub mod request;

use std::ffi::OsString;
use std::process::ExitCode;

pub fn run_cli<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match cli::run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
