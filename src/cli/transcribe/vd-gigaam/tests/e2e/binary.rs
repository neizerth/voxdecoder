//! End-to-end binary tests against the `vd-gigaam` executable.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn bin() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo_bin!("vd-gigaam"))
}

fn fixture_wav() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/audio/silence_0.2s.wav")
}

fn isolated_env(dir: &TempDir) -> (PathBuf, PathBuf) {
    let config = dir.path().join("config.toml");
    let models = dir.path().join("models");
    fs::create_dir_all(&models).unwrap();
    (config, models)
}

fn with_isolation(cmd: &mut assert_cmd::Command, config: &Path, models: &Path) {
    cmd.env("VD_GIGAAM_CONFIG", config)
        .env("VD_GIGAAM_MODELS_DIR", models);
}

#[test]
fn e2e_missing_input_exits_3() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    cmd.args(["run", "-i", "/no/such/file.wav", "--dry-run"])
        .assert()
        .failure()
        .code(3);
}

#[test]
fn e2e_o_and_d_conflict_exits_2() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    cmd.args([
        "run",
        "-i",
        wav.to_str().unwrap(),
        "-o",
        "out.txt",
        "-d",
        "outdir",
    ])
    .assert()
    .failure()
    .code(2);
}

#[test]
fn e2e_word_timestamps_without_sink_exits_2() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    cmd.args(["run", "-i", wav.to_str().unwrap(), "--word-timestamps"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn e2e_dry_run_text_plan() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    #[allow(unused_mut)]
    let mut args = vec![
        "run",
        "-i",
        wav.to_str().unwrap(),
        "-m",
        "ctc",
        "--dry-run",
    ];
    #[cfg(not(target_os = "macos"))]
    args.insert(args.len() - 2, "--flash");
    let assert = cmd.args(&args).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("Model: v2_ctc"), "{stdout}");
    #[cfg(not(target_os = "macos"))]
    assert!(stdout.contains("Flash: on"), "{stdout}");
    #[cfg(target_os = "macos")]
    assert!(stdout.contains("Flash: off"), "{stdout}");
    assert!(stdout.contains("FP16 encoder: on"), "{stdout}");
    assert!(stdout.contains("Overwrite: off"), "{stdout}");
    assert!(stdout.contains("Word timestamps: off"), "{stdout}");
    assert!(
        stdout.contains(&format!("Download root: {}", models.display())),
        "{stdout}"
    );
}

#[test]
fn e2e_dry_run_json_plan() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();
    let out = dir.path().join("result.json");
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    let assert = cmd
        .args([
            "run",
            "-i",
            wav.to_str().unwrap(),
            "-m",
            "v2_rnnt",
            "-o",
            out.to_str().unwrap(),
            "--segments",
            "--word-timestamps",
            "--format",
            "json",
            "--device",
            "cpu",
            "--dry-run",
            "--json",
        ])
        .assert()
        .success();
    let stdout = &assert.get_output().stdout;
    let plan: Value = serde_json::from_slice(stdout).expect("json plan");
    assert_eq!(plan["model"], "v2_rnnt");
    assert_eq!(plan["device"], "cpu");
    assert_eq!(plan["flash"], false);
    assert_eq!(plan["fp16_encoder"], true);
    assert_eq!(plan["overwrite"], false);
    assert_eq!(plan["word_timestamps"], true);
    assert_eq!(plan["output"].as_str().unwrap(), out.to_str().unwrap());
    assert_eq!(
        plan["segments"].as_str().unwrap(),
        dir.path().join("result.segments.json").to_str().unwrap()
    );
    assert_eq!(
        plan["download_root"].as_str().unwrap(),
        models.to_str().unwrap()
    );
}

#[test]
fn e2e_existing_output_without_overwrite_exits_2() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();
    let out = dir.path().join("meeting.txt");
    fs::write(&out, "old").unwrap();
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    cmd.args([
        "run",
        "-i",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--dry-run",
    ])
    .assert()
    .failure()
    .code(2);
}

#[test]
fn e2e_config_set_get_list_path() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);

    let mut set = bin();
    with_isolation(&mut set, &config, &models);
    set.args(["config", "set", "model", "v3_e2e_rnnt"])
        .assert()
        .success();

    let mut get = bin();
    with_isolation(&mut get, &config, &models);
    get.args(["config", "get", "model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("v3_e2e_rnnt"));

    let mut list = bin();
    with_isolation(&mut list, &config, &models);
    list.args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("model = v3_e2e_rnnt"));

    let mut path = bin();
    with_isolation(&mut path, &config, &models);
    path.args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains(config.to_str().unwrap()));
}

#[test]
fn e2e_config_overrides_dry_run_defaults() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();

    let mut set = bin();
    with_isolation(&mut set, &config, &models);
    set.args(["config", "set", "model", "v3_ctc"])
        .assert()
        .success();
    #[cfg(not(target_os = "macos"))]
    {
        let mut set_flash = bin();
        with_isolation(&mut set_flash, &config, &models);
        set_flash
            .args(["config", "set", "flash", "on"])
            .assert()
            .success();
    }
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    let assert = cmd
        .args([
            "run",
            "-i",
            wav.to_str().unwrap(),
            "--dry-run",
            "--json",
        ])
        .assert()
        .success();
    let plan: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(plan["model"], "v3_ctc");
    #[cfg(not(target_os = "macos"))]
    assert_eq!(plan["flash"], true);
    #[cfg(target_os = "macos")]
    assert_eq!(plan["flash"], false);
}

#[test]
fn e2e_list_and_info_json() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);

    // Pretend model is installed.
    fs::write(models.join("v2_rnnt.ckpt"), b"fake").unwrap();

    let mut list = bin();
    with_isolation(&mut list, &config, &models);
    list.args(["list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("v2_rnnt"))
        .stdout(predicate::str::contains("models_dir"));

    let mut info = bin();
    with_isolation(&mut info, &config, &models);
    let assert = info.args(["info", "v2_rnnt", "--json"]).assert().success();
    let meta: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(meta["name"], "v2_rnnt");
    assert_eq!(meta["decoder"], "rnnt");
    assert_eq!(meta["installed"], true);
    assert_eq!(meta["downloaded"], true);
}

#[test]
fn e2e_remove_requires_yes_or_fails_noninteractive() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let ckpt = models.join("v2_ctc.ckpt");
    fs::write(&ckpt, b"fake").unwrap();

    let mut rm = bin();
    with_isolation(&mut rm, &config, &models);
    // Non-TTY without --yes should refuse (exit 2).
    rm.args(["remove", "v2_ctc"]).assert().failure().code(2);
    assert!(ckpt.exists());

    let mut rm_yes = bin();
    with_isolation(&mut rm_yes, &config, &models);
    rm_yes.args(["remove", "v2_ctc", "-y"]).assert().success();
    assert!(!ckpt.exists());
}

#[test]
fn e2e_shorthand_dash_i_dry_run() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    cmd.args(["-i", wav.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Model: v2_rnnt"));
}

#[test]
fn e2e_transcribe_without_weights_exits_4() {
    let dir = TempDir::new().unwrap();
    let (config, models) = isolated_env(&dir);
    let wav = fixture_wav();
    let out = dir.path().join("out.txt");
    let mut cmd = bin();
    with_isolation(&mut cmd, &config, &models);
    cmd.args([
        "run",
        "-i",
        wav.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--device",
        "cpu",
    ])
    .assert()
    .failure()
    .code(4);
}

/// Full CTC transcription against golden text — needs published weights + fixtures.
#[test]
#[ignore = "requires installed GigaAM weights and golden transcript"]
fn e2e_golden_ctc_transcription() {
    let wav = fixture_wav();
    let expected = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/expected/silence_0.2s.txt");
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("out.txt");

    let status = Command::new(cargo_bin!("vd-gigaam"))
        .args([
            "run",
            "-i",
            wav.to_str().unwrap(),
            "-m",
            "v2_ctc",
            "-o",
            out.to_str().unwrap(),
            "--device",
            "cpu",
            "--overwrite",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let got = fs::read_to_string(&out).unwrap();
    let want = fs::read_to_string(expected).unwrap();
    assert_eq!(got.trim(), want.trim());
}

/// Smoke: load converted `models/v3_e2e_ctc` if present and transcribe fixture.
#[test]
fn e2e_ctc_converted_smoke_if_present() {
    let models = Path::new(env!("CARGO_MANIFEST_DIR")).join("models/v3_e2e_ctc");
    if !models.join("model.safetensors").is_file() {
        eprintln!("skip: no models/v3_e2e_ctc (run scripts/convert_ckpt.py)");
        return;
    }
    let wav = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/audio/silence_0.2s.wav");
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("out.txt");
    bin()
        .env("VD_GIGAAM_MODELS_DIR", models.parent().unwrap())
        .args([
            "run",
            "-i",
            wav.to_str().unwrap(),
            "-m",
            "v3_e2e_ctc",
            "-o",
            out.to_str().unwrap(),
            "--device",
            "cpu",
            "--overwrite",
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();
    assert!(out.is_file());
}

#[test]
fn e2e_install_already_converted_is_fast_ok() {
    let models = Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
    if !models.join("v3_e2e_ctc/model.safetensors").is_file() {
        eprintln!("skip: no models/v3_e2e_ctc");
        return;
    }
    bin()
        .args([
            "install",
            "v3_e2e_ctc",
            "--download-root",
            models.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .success();
}
