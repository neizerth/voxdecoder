//! Spawn `vd-meeting` binary.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-meeting"))
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_MEETING_CONFIG", config);
}

#[test]
fn dry_run_json_job() {
    let dir = TempDir::new().unwrap();
    let meeting = dir.path().join("meeting.yaml");
    fs::write(
        &meeting,
        "version: 1\nworking_dir: .\ninputs:\n  - role: merged\n    path: meeting.wav\nmeeting:\n  diarization:\n    enabled: auto\n",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    let stdout = cmd
        .arg(&meeting)
        .args(["--dry-run", "--json", "-q"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(v["version"], 1);
    let uses: Vec<&str> = v["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["use"].as_str().unwrap())
        .collect();
    assert!(uses.contains(&"transcribe"));
    assert!(uses.contains(&"diarize"));
    assert!(uses.contains(&"meeting-merge"));
}

#[test]
fn missing_input_on_run_exit_3() {
    let dir = TempDir::new().unwrap();
    let meeting = dir.path().join("meeting.yaml");
    fs::write(
        &meeting,
        "version: 1\ninputs:\n  - role: merged\n    path: /no/such/meeting.wav\n",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(&meeting)
        .arg("-q")
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("missing"));
}

#[test]
fn bad_role_exit_2() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args([
        "run",
        "--input",
        "role=tracks,path=a.wav",
        "--dry-run",
        "-q",
    ])
    .assert()
    .failure()
    .code(2);
}

#[test]
#[ignore = "requires VD_MEETING_E2E_FULL=1 and child CLIs"]
fn full_meeting_run() {}
