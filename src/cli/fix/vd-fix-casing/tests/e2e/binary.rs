//! Spawn `vd-fix-casing` binary; exit codes, I/O, progress, dry-run.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-fix-casing"))
}

fn with_isolation(cmd: &mut Command, config: &Path, models: &Path) {
    cmd.env("VD_FIX_CASING_CONFIG", config)
        .env("VD_FIX_CASING_MODELS_DIR", models);
}

fn install_ru(config: &Path, models: &Path) {
    let mut cmd = bin();
    with_isolation(&mut cmd, config, models);
    cmd.args(["install", "ru", "-q"]).assert().success();
}

#[test]
fn install_and_list() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["install", "ru", "--progress=json"])
        .assert()
        .success()
        .stderr(predicate::str::contains("\"event\":\"downloading\""));

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ru"))
        .stdout(predicate::str::contains("ready"));

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("ru"));
}

#[test]
fn run_without_install_uses_builtin() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("-q")
        .assert()
        .success();
    let out = dir.path().join("meeting.fixed.txt");
    assert!(out.is_file());
}

#[test]
fn dry_run_shows_builtin_when_not_installed() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Model: ru"))
        .stdout(predicate::str::contains("Pack installed: no (builtin)"));
}

#[test]
fn dry_run_shows_installed() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");
    install_ru(&cfg, &models);

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Model: ru"))
        .stdout(predicate::str::contains("Pack installed: yes"));
}

#[test]
fn dry_run_json() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");
    install_ru(&cfg, &models);

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
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
    assert_eq!(v["model"], "ru");
    assert_eq!(v["installed"], true);
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
fn unsupported_type_exit_3() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.wav");
    fs::write(&input, "x").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"]).arg(&input).assert().code(3);
}

#[test]
fn run_writes_fixed_txt() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "мы обсуждали кубернетис").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");
    install_ru(&cfg, &models);

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("-q")
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.txt");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text, "Мы обсуждали кубернетис.");
}

#[test]
fn run_progress_has_processing_percent() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "мы обсуждали кубернетис").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");
    install_ru(&cfg, &models);

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--progress=json")
        .assert()
        .success()
        .stderr(predicate::str::contains("\"event\":\"processing\""))
        .stderr(predicate::str::contains("\"percent\""));
}

#[test]
fn config_roundtrip() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["config", "set", "language", "en"])
        .assert()
        .success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["config", "get", "language"])
        .assert()
        .success()
        .stdout(predicate::str::contains("en"));
}

#[test]
fn conflict_o_d_exit_2() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.txt");
    fs::write(&input, "x").unwrap();
    let cfg = dir.path().join("config.toml");
    let models = dir.path().join("models");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &models);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["-o", "b.txt", "-d", "out"])
        .assert()
        .code(2);
}
