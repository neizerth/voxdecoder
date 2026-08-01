//! `vd-diarize` — local-first speaker diarization (CLI ≡ `use: diarize`).

pub mod artifact;
pub mod assets;
pub mod backend;
pub mod cli;
pub mod config;
pub mod paths;
pub mod run;
pub mod status;

pub use artifact::{
    AudioRef, BackendInfo, Embeddings, SpeakerEmbedding, SpeakerId, SpeakerTimeline, Segment,
};
pub use backend::{BackendSpec, DiarizeError, DiarizeRequest};
pub use run::diarize;

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
