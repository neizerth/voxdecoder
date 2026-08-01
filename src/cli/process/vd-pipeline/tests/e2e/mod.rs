#![allow(clippy::default_trait_access)]
//! E2E: spawn `vd-pipeline` binary.

mod bad_job;
mod dry_run;
mod missing_input;
mod run_light;
mod whisper;

use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-pipeline"))
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(rel: &str) -> PathBuf {
    fixtures_dir().join(rel)
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_PIPELINE_CONFIG", config);
    cmd.env_remove("VD_PROJECT_DIR");
}
