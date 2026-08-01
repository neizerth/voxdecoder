//! Spawn `vd-fix-asr` binary; exit codes, I/O, progress, dry-run.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-fix-asr"))
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_FIX_ASR_CONFIG", config);
}

#[test]
fn run_writes_fixed_txt() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "мы используем гитхап экшенс").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("-q")
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.txt");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text, "мы используем гитхаб экшенс");
}

#[test]
fn dry_run_shows_context() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let docs = dir.path().join("docs");
    fs::create_dir(&docs).unwrap();
    fs::write(docs.join("a.md"), "github").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--context")
        .arg(&docs)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Language: ru"))
        .stdout(predicate::str::contains("Context:"));
}

#[test]
fn dry_run_json() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    let out = cmd
        .args(["-i"])
        .arg(&input)
        .args(["--dry-run", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["artifact_type"], "txt");
    assert_eq!(v["language"], "ru");
    assert_eq!(v["context_neighbors"], 1);
}

#[test]
fn run_progress_has_span_percent() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "мы используем гитхап экшенс").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--progress=json")
        .assert()
        .success()
        .stderr(predicate::str::contains("\"event\":\"processing\""))
        .stderr(predicate::str::contains("\"span\""))
        .stderr(predicate::str::contains("\"percent\""));
}

#[test]
fn missing_input_exit_3() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i", "/no/such/file.txt"])
        .assert()
        .code(3);
}

#[test]
fn unsupported_type_exit_3() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.wav");
    fs::write(&input, "x").unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"]).arg(&input).assert().code(3);
}

#[test]
fn conflict_o_d_exit_2() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.txt");
    fs::write(&input, "x").unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["-o", "b.txt", "-d", "out"])
        .assert()
        .code(2);
}

#[test]
fn config_roundtrip() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["config", "set", "context_neighbors", "2"])
        .assert()
        .success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["config", "get", "context_neighbors"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}
