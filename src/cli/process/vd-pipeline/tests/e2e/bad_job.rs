//! Invalid job / unknown artifact.

use predicates::prelude::*;
use tempfile::TempDir;

use super::{bin, fixture, with_isolation};

#[test]
fn bad_use_exit_2() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(fixture("jobs/bad_use.yaml"))
        .args(["--dry-run", "-q"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn bad_artifact_exit_2() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(fixture("jobs/bad_artifact.yaml"))
        .args(["--dry-run", "-q"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unknown artifact id"));
}
