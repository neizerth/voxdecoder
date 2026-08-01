//! Whisper reserved → exit 2.

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use super::{bin, fixture, with_isolation};

#[test]
fn whisper_cli_exit_2() {
    let dir = TempDir::new().unwrap();
    let audio = dir.path().join("meeting.ogg");
    fs::write(&audio, b"fake").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["-i"])
        .arg(&audio)
        .args(["--asr", "whisper", "--dry-run", "-q"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("whisper").or(predicate::str::contains("reserved")));
}

#[test]
fn whisper_job_file_exit_2() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(fixture("jobs/whisper.yaml"))
        .args(["--dry-run", "-q"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("whisper"));
}
