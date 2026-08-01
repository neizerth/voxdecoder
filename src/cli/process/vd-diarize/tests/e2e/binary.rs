//! Spawn `vd-diarize` binary.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-diarize"))
}

fn with_isolation(cmd: &mut Command, config: &Path, assets: &Path) {
    cmd.env("VD_DIARIZE_CONFIG", config);
    cmd.env("VD_DIARIZE_ASSETS", assets);
}

#[test]
fn run_stub_produces_json() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.wav");
    fs::write(&input, vec![1u8; 8_000]).unwrap();
    let cfg = dir.path().join("config.toml");
    let assets = dir.path().join("assets");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &assets);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["--backend", "stub", "-q", "--overwrite"])
        .assert()
        .success()
        .stdout(predicate::str::contains("diarization.json"));

    let out = dir.path().join("a.diarization.json");
    assert!(out.exists());
    let text = fs::read_to_string(&out).unwrap();
    assert!(text.contains("\"provider\": \"stub\"") || text.contains("\"provider\":\"stub\""));
}

#[test]
fn dry_run_json() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.wav");
    fs::write(&input, b"x").unwrap();
    let cfg = dir.path().join("config.toml");
    let assets = dir.path().join("assets");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &assets);
    let stdout = cmd
        .args(["-i"])
        .arg(&input)
        .args(["--backend", "stub", "--dry-run", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(v["backend"]["provider"], "stub");
}

#[test]
fn missing_input_exit_3() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let assets = dir.path().join("assets");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &assets);
    cmd.args(["run", "-i", "/no/such/file.wav", "-q"])
        .assert()
        .failure()
        .code(3);
}

#[test]
fn install_list_stub() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let assets = dir.path().join("assets");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &assets);
    cmd.args(["install", "stub"]).assert().success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg, &assets);
    cmd.arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("stub"));
}
