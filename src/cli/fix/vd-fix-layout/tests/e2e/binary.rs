//! Spawn `vd-fix-layout` binary; exit codes, I/O, progress, dry-run.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-fix-layout"))
}

fn with_isolation(cmd: &mut Command, config: &Path, models: &Path) {
    cmd.env("VD_FIX_LAYOUT_CONFIG", config)
        .env("VD_FIX_LAYOUT_MODELS_DIR", models);
}

#[test]
fn run_without_install_uses_builtin() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(
        &input,
        "Баня. Баня — это место. Самое главное — это веник. После бани чай.",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("-l")
        .arg("ru")
        .arg("-q")
        .assert()
        .success();
    let out = dir.path().join("meeting.fixed.txt");
    assert!(out.is_file());
}

#[test]
fn dry_run_json_has_abstract_timemap() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "Hello world. Another sentence here for detection.").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    let out = cmd
        .args(["-i"])
        .arg(&input)
        .args(["--language", "auto", "--dry-run", "--json", "--no-timemap"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["language"], "auto");
    assert!(v.get("language_resolved").is_some());
    assert!(v["timemap"].is_null());
    // Must not promise a filesystem path field for TimeMap.
    assert!(v.get("timemap_path").is_none());
}

#[test]
fn dry_run_reports_phases_language() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "текст на русском языке для автоопределения языка.").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["--language", "auto", "--dry-run", "--no-timemap"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Language: auto"))
        .stdout(predicate::str::contains("Language resolved:"))
        .stdout(predicate::str::contains("TimeMap:"));
}

#[test]
fn missing_input_exit_3() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i", "/no/such/file.txt"])
        .assert()
        .code(3);
}

#[test]
fn run_progress_has_analyzing_and_layout() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(
        &input,
        "One. Two. Three. Anyway four. Five. Six.",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["-l", "en", "--progress=json"])
        .assert()
        .success()
        .stderr(predicate::str::contains("\"phase\":\"analyzing\""))
        .stderr(predicate::str::contains("\"phase\":\"layout\""));
}

#[test]
fn config_roundtrip() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["config", "set", "language", "auto"])
        .assert()
        .success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["config", "get", "language"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto"));
}

#[test]
fn install_and_list() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["install", "ru", "-q"]).assert().success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ru"))
        .stdout(predicate::str::contains("ready"));
}
