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
    cmd.env_remove("VD_PROJECT_DIR");
}

#[test]
fn run_writes_fixed_txt() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "мы используем гитхап").unwrap();
    let dict = dir.path().join("asr.yml");
    fs::write(&dict, "canonical: гитхаб\nvariants:\n  - гитхап\n").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--dictionary")
        .arg(&dict)
        .arg("-q")
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.txt");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text, "мы используем гитхаб");
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
fn defaults_context_to_dot_voxdecoder() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "hello").unwrap();
    let assets = dir.path().join(".voxdecoder");
    fs::create_dir(&assets).unwrap();
    fs::write(
        assets.join("terms.yml"),
        "version: 1\nentries: []\nforms: []\n",
    )
    .unwrap();
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
fn report_counts_dictionary_hit() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "мы используем гитхап").unwrap();
    let dict = dir.path().join("asr.yml");
    fs::write(&dict, "canonical: гитхаб\nvariants:\n  - гитхап\n").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    let out = cmd
        .args(["run", "-i"])
        .arg(&input)
        .arg("--dictionary")
        .arg(&dict)
        .args(["-q", "--report"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["dictionary"], 1);
    assert_eq!(v["unsafe"], 0);
}

#[test]
fn strict_and_aggressive_conflict_exit_2() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.txt");
    fs::write(&input, "x").unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["--strict", "--aggressive"])
        .assert()
        .code(2);
}

#[test]
fn strict_suppresses_likely_within_token_doubling() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "каккак дела").unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["-q", "--strict"])
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.txt");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(
        text, "каккак дела",
        "Likely-confidence fix must not apply under --strict"
    );
}

#[test]
fn dictionary_flag_applies_custom_entry() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "foobaz project").unwrap();
    let terms = dir.path().join("terms.yml");
    fs::write(
        &terms,
        "version: 1\nentries:\n  - canonical: foobar\n    variants: [foobaz]\nforms: []\n",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--dictionary")
        .arg(&terms)
        .arg("-q")
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.txt");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text, "foobar project");
}

#[test]
fn project_flag_reads_voxdecoder_dictionary() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "quuxx project").unwrap();
    let project_dir = dir.path().join("proj");
    let voxdecoder = project_dir.join(".voxdecoder");
    fs::create_dir_all(&voxdecoder).unwrap();
    fs::write(
        voxdecoder.join("asr-dictionary.yml"),
        "version: 1\nentries:\n  - canonical: quux\n    variants: [quuxx]\nforms: []\n",
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("--project")
        .arg(&project_dir)
        .arg("-q")
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.txt");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text, "quux project");
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
