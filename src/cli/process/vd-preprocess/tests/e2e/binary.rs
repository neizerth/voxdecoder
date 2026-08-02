//! Spawn `vd-preprocess` binary.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-preprocess"))
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_PREPROCESS_CONFIG", config);
}

#[test]
fn no_filters_exit_2() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let input = dir.path().join("a.wav");
    fs::write(&input, b"x").unwrap();
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i", input.to_str().unwrap(), "-q"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("no filters"));
}

#[test]
fn dry_run_and_stub_run() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("in.wav");
    fs::write(&input, b"audio-bytes").unwrap();
    let cfg = dir.path().join("config.toml");
    fs::write(&cfg, "provider = \"stub\"\n").unwrap();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.current_dir(dir.path())
        .args([
            "run",
            "-i",
            input.to_str().unwrap(),
            "--filter",
            "normalize",
            "--filter",
            "mono",
            "--provider",
            "stub",
            "-d",
            dir.path().to_str().unwrap(),
            "--dry-run",
            "--json",
            "-q",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("normalize"));

    let out = dir.path().join("in.prepared.wav");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.current_dir(dir.path())
        .args([
            "run",
            "-i",
            input.to_str().unwrap(),
            "--filter",
            "normalize",
            "--provider",
            "stub",
            "-o",
            out.to_str().unwrap(),
            "--overwrite",
            "-q",
        ])
        .assert()
        .success();
    assert!(out.exists());
}
