//! Spawn `vd-postprocess` binary.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-postprocess"))
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_POSTPROCESS_CONFIG", config);
}

#[test]
fn no_recipes_exit_2() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "--input", "a=x.txt", "-q"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("no recipes"));
}

#[test]
fn dry_run_and_stub_run() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("in.txt");
    fs::write(&input, "payload").unwrap();
    let recipe = dir.path().join("r.yaml");
    fs::write(
        &recipe,
        "version: 1\nid: s\ninputs:\n  transcript:\n    required: true\noutputs:\n  - id: summary\n    path: out.md\nprompt: |\n  {{ transcript }}\n",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.current_dir(dir.path())
        .args([
            "run",
            "--input",
            &format!("transcript={}", input.display()),
            "--recipe",
            recipe.to_str().unwrap(),
            "--runner",
            "stub",
            "-d",
            dir.path().to_str().unwrap(),
            "--dry-run",
            "--json",
            "-q",
        ])
        .assert()
        .success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.current_dir(dir.path())
        .args([
            "run",
            "--input",
            &format!("transcript={}", input.display()),
            "--recipe",
            recipe.to_str().unwrap(),
            "--provider", // alias
            "stub",
            "-d",
            dir.path().to_str().unwrap(),
            "--overwrite",
            "-q",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("summary:"));

    assert!(dir.path().join("out.md").exists());
}

#[test]
fn run_progress_json() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("in.txt");
    fs::write(&input, "payload").unwrap();
    let recipe = dir.path().join("r.yaml");
    fs::write(
        &recipe,
        "version: 1\nid: s\ninputs:\n  transcript:\n    required: true\noutputs:\n  - id: summary\n    path: out.md\nprompt: |\n  {{ transcript }}\n",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.current_dir(dir.path())
        .args([
            "run",
            "--input",
            &format!("transcript={}", input.display()),
            "--recipe",
            recipe.to_str().unwrap(),
            "--runner",
            "stub",
            "-d",
            dir.path().to_str().unwrap(),
            "--overwrite",
            "--progress=json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("\"event\":\"start\""))
        .stderr(predicate::str::contains("\"event\":\"phase\""))
        .stderr(predicate::str::contains("\"phase\":\"planning\""))
        .stderr(predicate::str::contains("\"phase\":\"executing\""))
        .stderr(predicate::str::contains("\"event\":\"done\""));
}
