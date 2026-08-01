//! Spawn `vd-assets` binary.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-assets"))
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_ASSETS_CONFIG", config);
    cmd.env("VD_ASSETS_CACHE", config.with_extension("cache"));
    cmd.env_remove("VD_PROJECT_DIR");
}

#[test]
fn run_from_markdown() {
    let dir = TempDir::new().unwrap();
    let md = dir.path().join("notes.md");
    fs::write(&md, "AcmeCloud deploy\n").unwrap();
    let out = dir.path().join("assets");
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&md)
        .arg("-o")
        .arg(&out)
        .arg("-q")
        .assert()
        .success()
        .stdout(predicate::str::contains("terms.yml"));

    assert!(out.join("terms.yml").exists());
    assert!(out.join("md").join("notes.md").exists());
}

#[test]
fn dry_run_json() {
    let dir = TempDir::new().unwrap();
    let md = dir.path().join("a.md");
    fs::write(&md, "x").unwrap();
    let out = dir.path().join("out");
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    let stdout = cmd
        .args(["-i"])
        .arg(&md)
        .arg("-o")
        .arg(&out)
        .args(["--dry-run", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert!(v["terms"].as_str().unwrap().contains("terms.yml"));
}

#[test]
fn missing_input_exit_3() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args([
        "run",
        "-i",
        "/no/such",
        "-o",
        dir.path().join("o").to_str().unwrap(),
    ])
    .assert()
    .code(3);
}
