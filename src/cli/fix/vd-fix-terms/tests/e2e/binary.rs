//! Spawn `vd-fix-terms` binary; exit codes, I/O, progress, dry-run.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-fix-terms"))
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_FIX_TERMS_CONFIG", config);
    cmd.env_remove("VD_PROJECT_DIR");
}

#[test]
fn run_writes_fixed_txt() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "мы деплоим на кубернетис и гоняем гитхап экшенс").unwrap();
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
    assert_eq!(text, "мы деплоим на Kubernetes и гоняем GitHub Actions");
}

#[test]
fn corp_terms_override_and_disable_shipping() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "берём акмеклауд и k8s").unwrap();
    let terms = dir.path().join("corp.yaml");
    fs::write(
        &terms,
        "canonical: AcmeCloud\nvariants:\n  - акмеклауд\n---\ncanonical: OurK8s\nvariants:\n  - k8s\n",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--terms")
        .arg(&terms)
        .arg("--no-shipping-lexicon")
        .arg("-q")
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.txt");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text, "берём AcmeCloud и OurK8s");
}

#[test]
fn dry_run_shows_terms() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let terms = dir.path().join("g.yaml");
    fs::write(&terms, "canonical: X\nvariants:\n  - x\n").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--terms")
        .arg(&terms)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Terms:"));
}

#[test]
fn defaults_terms_to_dot_voxdecoder() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let assets = dir.path().join(".voxdecoder");
    fs::create_dir(&assets).unwrap();
    fs::write(assets.join("terms.yml"), "version: 1\nentries: []\nforms: []\n").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains(".voxdecoder"));
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
        .args(["--dry-run", "--json", "--no-shipping-lexicon"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["artifact_type"], "txt");
    assert_eq!(v["language"], "ru");
    assert_eq!(v["shipping_lexicon"], false);
}

#[test]
fn run_progress_has_span_percent() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "кубернетис").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--progress=json")
        .assert()
        .success()
        .stderr(predicate::str::contains("\"event\":\"phase\""))
        .stderr(predicate::str::contains("\"phase\":\"processing\""))
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
fn missing_terms_exit_3() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.txt");
    fs::write(&input, "x").unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["--terms", "/no/such/terms.yaml"])
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
    cmd.args(["config", "set", "in_place", "on"])
        .assert()
        .success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["config", "get", "in_place"])
        .assert()
        .success()
        .stdout(predicate::str::contains("on"));
}
