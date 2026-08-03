//! Spawn `vd-fix-overlap` binary; exit codes, I/O, report formats.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(cargo_bin!("vd-fix-overlap"))
}

fn with_isolation(cmd: &mut Command, config: &Path) {
    cmd.env("VD_FIX_OVERLAP_CONFIG", config);
}

const TURNS_WITH_DUP: &str = r#"{
  "turns": [
    {"speaker": "A", "text": "Let's deploy tomorrow.", "start_sec": 1.0, "end_sec": 3.0},
    {"speaker": "B", "text": "let's deploy tomorrow",  "start_sec": 1.2, "end_sec": 3.2},
    {"speaker": "A", "text": "Sounds good to me.",      "start_sec": 3.2, "end_sec": 4.0}
  ]
}"#;

const TURNS_NO_DUP: &str = r#"{
  "turns": [
    {"speaker": "A", "text": "Let's deploy tomorrow.", "start_sec": 0.0, "end_sec": 1.0},
    {"speaker": "B", "text": "Sounds good to me.",      "start_sec": 1.0, "end_sec": 2.0}
  ]
}"#;

#[test]
fn run_reports_duplicate_text() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("turns.json");
    fs::write(&input, TURNS_WITH_DUP).unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("1 candidate duplicate pair"))
        .stdout(predicate::str::contains("[exact]"));
}

#[test]
fn run_reports_no_duplicates() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("turns.json");
    fs::write(&input, TURNS_NO_DUP).unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("No duplicate speech detected"));
}

#[test]
fn run_json_report_is_well_formed() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("turns.json");
    fs::write(&input, TURNS_WITH_DUP).unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    let out = cmd
        .args(["-i"])
        .arg(&input)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["kind"], "exact");
    assert_eq!(arr[0]["keep"], 0);
    assert_eq!(arr[0]["drop"], 1);
}

#[test]
fn strict_threshold_via_cli_suppresses_near_matches() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("turns.json");
    fs::write(
        &input,
        r#"{"turns": [
            {"speaker": "A", "text": "Deploy tomorrow morning.", "start_sec": 0.0, "end_sec": 2.0},
            {"speaker": "B", "text": "Deploy tomorrow mornin", "start_sec": 0.1, "end_sec": 2.1}
        ]}"#,
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["--similarity-threshold", "0.99"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No duplicate speech detected"));
}

#[test]
fn apply_removes_duplicate_turn_and_writes_fixed_json() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.json");
    fs::write(&input, TURNS_WITH_DUP).unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["-q", "--apply"])
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.json");
    let text = fs::read_to_string(&out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let turns = v["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2, "one duplicate turn should be removed");
    assert_eq!(turns[0]["speaker"], "A");
    assert_eq!(turns[1]["speaker"], "A");
    assert_eq!(turns[1]["text"], "Sounds good to me.");
}

#[test]
fn apply_trims_partial_duplicate_instead_of_deleting_unique_tail() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.json");
    fs::write(
        &input,
        r#"{"turns": [
            {"speaker": "A", "start_sec": 0.0, "end_sec": 2.0, "text": "Deploy tomorrow morning"},
            {"speaker": "B", "start_sec": 0.1, "end_sec": 2.1, "text": "Deploy tomorrow morning ok"}
        ]}"#,
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["-q", "--apply"])
        .assert()
        .success();

    let out = dir.path().join("meeting.fixed.json");
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    let turns = v["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2, "B must survive, trimmed — not be deleted");
    assert_eq!(turns[1]["speaker"], "B");
    assert_eq!(turns[1]["text"], "ok");
    assert_eq!(
        turns[1]["start_sec"], 0.1,
        "trimming text must not touch other fields"
    );
}

#[test]
fn output_flag_implies_apply() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.json");
    fs::write(&input, TURNS_WITH_DUP).unwrap();
    let out_path = dir.path().join("cleaned.json");
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .arg("-o")
        .arg(&out_path)
        .arg("-q")
        .assert()
        .success();

    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    assert_eq!(v["turns"].as_array().unwrap().len(), 2);
}

#[test]
fn apply_with_no_duplicates_does_not_write() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.json");
    fs::write(&input, TURNS_NO_DUP).unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["-q", "--apply"])
        .assert()
        .success();

    assert!(!dir.path().join("meeting.fixed.json").exists());
}

#[test]
fn no_speaker_turns_found_exit_3() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("plain.txt");
    fs::write(&input, "just plain text, no turns").unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"]).arg(&input).assert().code(3);
}

#[test]
fn missing_input_exit_3() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i", "/no/such/file.json"])
        .assert()
        .code(3);
}

#[test]
fn invalid_json_exit_1() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("turns.json");
    fs::write(&input, "not json").unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"]).arg(&input).assert().code(1);
}

#[test]
fn out_of_range_threshold_exit_2() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("turns.json");
    fs::write(&input, TURNS_NO_DUP).unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-i"])
        .arg(&input)
        .args(["--similarity-threshold", "2.0"])
        .assert()
        .code(2);
}

#[test]
fn config_roundtrip() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["config", "set", "max_gap_ms", "1000"])
        .assert()
        .success();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["config", "get", "max_gap_ms"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1000"));
}

#[test]
fn config_path_prints_a_path() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}
