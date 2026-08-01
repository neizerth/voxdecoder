#![allow(clippy::default_trait_access)]
//! Unit tests for `vd-pipeline` (no process spawn).

mod artifacts;
mod cli;
mod default_job;
mod engine_gate;
mod job_parse;
mod postprocess;
mod resolve;
mod status;

use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(rel: &str) -> PathBuf {
    fixtures_dir().join(rel)
}
