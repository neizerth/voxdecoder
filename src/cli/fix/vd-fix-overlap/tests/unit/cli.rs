//! CLI parsing and validation.

use std::path::PathBuf;

use vd_fix_overlap::cli::{parse_args, CliError, Command as VdCommand};

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-fix-overlap"];
    full.extend(args);
    parse_args(full)
}

#[test]
fn shorthand_dash_i_is_run() {
    let cmd = parse(&["-i", "turns.json"]).unwrap();
    match cmd {
        VdCommand::Run(r) => assert_eq!(r.input, PathBuf::from("turns.json")),
        VdCommand::Config(_) => panic!("expected Run"),
    }
}

#[test]
fn run_defaults() {
    let cmd = parse(&["run", "-i", "turns.json"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert!(r.similarity_threshold.is_none());
    assert!(r.max_gap_ms.is_none());
    assert!(!r.json);
    assert!(!r.quiet);
}

#[test]
fn threshold_and_gap_flags_parse() {
    let cmd = parse(&[
        "run",
        "-i",
        "turns.json",
        "--similarity-threshold",
        "0.9",
        "--max-gap-ms",
        "250",
    ])
    .unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.similarity_threshold, Some(0.9));
    assert_eq!(r.max_gap_ms, Some(250));
}

#[test]
fn out_of_range_threshold_rejected() {
    let err = parse(&["run", "-i", "turns.json", "--similarity-threshold", "1.5"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn missing_input_flag_is_usage_error() {
    let err = parse(&["run"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}
