//! Dry-run e2e.

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use super::{bin, fixture, with_isolation};

#[test]
fn dry_run_default_json() {
    let dir = TempDir::new().unwrap();
    let audio = dir.path().join("meeting.ogg");
    fs::write(&audio, b"fake").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    let stdout = cmd
        .args(["-i"])
        .arg(&audio)
        .args(["--dry-run", "--json", "-q"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 4);
    assert_eq!(steps[0]["use"], "transcribe");
    assert_eq!(steps[0]["id"], "transcript");
    assert!(!steps.iter().any(|s| s["use"] == "prepare-context"));
    assert_eq!(steps[1]["use"], "fix-casing");
}

#[test]
fn dry_run_with_docs() {
    let dir = TempDir::new().unwrap();
    let audio = dir.path().join("meeting.ogg");
    fs::write(&audio, b"fake").unwrap();
    let docs = dir.path().join("docs");
    fs::create_dir(&docs).unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    let stdout = cmd
        .args(["-i"])
        .arg(&audio)
        .arg("--docs")
        .arg(&docs)
        .args(["--dry-run", "--json", "-q"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    let steps = v["steps"].as_array().unwrap();
    assert!(steps.iter().any(|s| s["use"] == "prepare-context"));
}

#[test]
fn dry_run_file() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(fixture("jobs/full.yaml"))
        .args(["--dry-run", "--json", "-q"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prepare-context"))
        .stdout(predicate::str::contains("Initial transcript"));
}

#[test]
fn cli_equals_file_shape() {
    let dir = TempDir::new().unwrap();
    let audio = dir.path().join("meeting.ogg");
    fs::write(&audio, b"fake").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cli = bin();
    with_isolation(&mut cli, &cfg);
    let cli_out = cli
        .args(["-i"])
        .arg(&audio)
        .args(["--dry-run", "--json", "-q"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cli_v: serde_json::Value = serde_json::from_slice(&cli_out).unwrap();

    let fixture_text = fs::read_to_string(fixture("jobs/default.json")).unwrap();
    let file_v: serde_json::Value = serde_json::from_str(&fixture_text).unwrap();

    assert_eq!(cli_v["steps"], file_v["steps"]);
}
