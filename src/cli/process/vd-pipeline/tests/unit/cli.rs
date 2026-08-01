//! CLI argv parsing.

use std::ffi::OsString;
use std::path::PathBuf;

use vd_pipeline::cli::{parse_args, Command};
use vd_pipeline::progress::ProgressMode;

fn args(parts: &[&str]) -> Vec<OsString> {
    let mut v = vec![OsString::from("vd-pipeline")];
    v.extend(parts.iter().map(OsString::from));
    v
}

#[test]
fn shorthand_input_inserts_run() {
    let cmd = parse_args(args(&["-i", "meeting.ogg"])).unwrap();
    match cmd {
        Command::Run(r) => {
            assert_eq!(r.input, Some(PathBuf::from("meeting.ogg")));
            assert!(r.job_file.is_none());
        }
        Command::Config(_) => panic!("expected Run"),
    }
}

#[test]
fn run_with_asr_model_docs() {
    let cmd = parse_args(args(&[
        "run",
        "-i",
        "a.ogg",
        "--asr",
        "gigaam",
        "-m",
        "v2_rnnt",
        "--docs",
        "./docs",
        "--dry-run",
        "--json",
        "--progress=json",
    ]))
    .unwrap();
    match cmd {
        Command::Run(r) => {
            assert_eq!(r.asr, "gigaam");
            assert_eq!(r.model.as_deref(), Some("v2_rnnt"));
            assert_eq!(r.docs, Some(PathBuf::from("./docs")));
            assert!(r.dry_run && r.json);
            assert_eq!(r.progress, Some(ProgressMode::Json));
        }
        Command::Config(_) => panic!("expected Run"),
    }
}

#[test]
fn job_file_and_input_conflict() {
    let err = parse_args(args(&["run", "-i", "a.ogg", "job.yaml"])).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.message().contains("mutually exclusive"));
}

#[test]
fn json_requires_dry_run() {
    let err = parse_args(args(&["run", "-i", "a.ogg", "--json"])).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn missing_input_exit_3() {
    let err = parse_args(args(&["run"])).unwrap_err();
    assert_eq!(err.exit_code(), 3);
}

#[test]
fn positional_job_file() {
    let cmd = parse_args(args(&["run", "job.yaml", "--dry-run"])).unwrap();
    match cmd {
        Command::Run(r) => {
            assert_eq!(r.job_file, Some(PathBuf::from("job.yaml")));
            assert!(r.input.is_none());
        }
        Command::Config(_) => panic!("expected Run"),
    }
}
