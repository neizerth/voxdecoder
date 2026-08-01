//! CLI parsing, validation, and exit-code contracts from `cli.md`.

use std::path::PathBuf;
use std::process::Command;

use vd_gigaam::cli::{parse_args, CliError, Command as VdCommand, ProgressMode};
use vd_gigaam::config::resolve::{Device, OutputFormat};
use vd_gigaam::gigaam::catalog::resolve_model_name;

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-gigaam"];
    full.extend(args);
    parse_args(full)
}

#[test]
fn shorthand_dash_i_is_run() {
    let cmd = parse(&["-i", "a.wav"]).unwrap();
    match cmd {
        VdCommand::Run(r) => assert_eq!(r.input, PathBuf::from("a.wav")),
        other => panic!("expected Run, got {other:?}"),
    }
}

#[test]
fn run_defaults() {
    let cmd = parse(&["run", "-i", "a.wav"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.model.as_deref(), None);
    assert_eq!(r.device, None);
    assert!(!r.flash);
    assert!(!r.no_fp16_encoder);
    assert!(!r.word_timestamps);
    assert!(!r.overwrite);
    assert!(!r.dry_run);
    assert!(!r.json);
    assert_eq!(r.format, None);
    assert_eq!(r.progress, None);
    assert!(!r.quiet);
    assert!(!r.segments);
}

#[test]
fn run_rejects_output_and_output_dir() {
    let err = parse(&["run", "-i", "a.wav", "-o", "out.txt", "-d", "dir"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn run_rejects_word_timestamps_without_sink() {
    let err = parse(&["run", "-i", "a.wav", "--word-timestamps"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);

    let err = parse(&["run", "-i", "a.wav", "--word-timestamps", "--format", "txt"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);

    assert!(parse(&[
        "run",
        "-i",
        "a.wav",
        "--word-timestamps",
        "--format",
        "json"
    ])
    .is_ok());
    assert!(parse(&["run", "-i", "a.wav", "--word-timestamps", "--segments"]).is_ok());
}

#[test]
fn progress_text_by_default() {
    let cmd = parse(&["run", "-i", "a.wav"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.effective_progress(), ProgressMode::Text);
}

#[test]
fn quiet_disables_progress() {
    let cmd = parse(&["run", "-i", "a.wav", "-q"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.effective_progress(), ProgressMode::None);
}

#[test]
fn progress_json_explicit() {
    let cmd = parse(&["run", "-i", "a.wav", "--progress=json"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.effective_progress(), ProgressMode::Json);
}

#[test]
fn quiet_overrides_progress_json() {
    let cmd = parse(&["run", "-i", "a.wav", "--progress=json", "-q"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.effective_progress(), ProgressMode::None);
}

#[test]
fn model_aliases() {
    assert_eq!(resolve_model_name("rnnt"), "v2_rnnt");
    assert_eq!(resolve_model_name("ctc"), "v2_ctc");
    assert_eq!(resolve_model_name("e2e_rnnt"), "v3_e2e_rnnt");
    assert_eq!(resolve_model_name("e2e_ctc"), "v3_e2e_ctc");
    assert_eq!(resolve_model_name("v2_rnnt"), "v2_rnnt");
}

#[test]
fn config_subcommands_parse() {
    assert!(matches!(
        parse(&["config", "list"]).unwrap(),
        VdCommand::Config(_)
    ));
    assert!(matches!(
        parse(&["config", "get", "model"]).unwrap(),
        VdCommand::Config(_)
    ));
    assert!(matches!(
        parse(&["config", "set", "flash", "on"]).unwrap(),
        VdCommand::Config(_)
    ));
    assert!(matches!(
        parse(&["config", "path"]).unwrap(),
        VdCommand::Config(_)
    ));
}

#[test]
fn install_requires_model_or_all() {
    let err = parse(&["install"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    let msg = err.to_string();
    assert!(msg.contains("v3_e2e_ctc"), "{msg}");
    assert!(msg.contains("Aliases:"), "{msg}");
    assert!(parse(&["install", "--all"]).is_ok());
    assert!(parse(&["install", "v2_rnnt"]).is_ok());
    assert!(parse(&["install", "ctc"]).is_ok());
}

#[test]
fn install_rejects_unknown_model() {
    let err = parse(&["install", "nope"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("unknown model 'nope'"));
}

#[test]
fn device_and_format_enums() {
    let cmd = parse(&["run", "-i", "a.wav", "--device", "cpu", "--format", "srt"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.device, Some(Device::Cpu));
    assert_eq!(r.format, Some(OutputFormat::Srt));
}

#[test]
fn binary_unknown_flag_exits_2() {
    let bin = env!("CARGO_BIN_EXE_vd-gigaam");
    let out = Command::new(bin)
        .args(["run", "-i", "a.wav", "--not-a-real-flag"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(2));
}
