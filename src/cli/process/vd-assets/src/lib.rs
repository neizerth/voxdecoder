//! `vd-assets` — convert project docs to Markdown and build a dictionary for `vd-fix-*`.

pub mod cli;
pub mod config;
pub mod convert;
pub mod dict;
pub mod paths;
pub mod types;

pub use convert::OcrMode;
pub use dict::{
    is_assets_dir, load_dictionary, write_dictionary, write_lexicon, write_terms, Dictionary,
    DictionaryError, DictionaryOptions, TermEntry, MD_DIR, TERMS_NAME,
};

pub use vd_artifact as artifact;
pub use vd_output as output;
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
