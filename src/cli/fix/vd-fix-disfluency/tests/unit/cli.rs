//! CLI parsing and validation.

use std::path::PathBuf;

use vd_fix_disfluency::cli::{parse_args, CliError, Command as VdCommand};
use vd_fix_disfluency::types::{Language, Mode};

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-fix-disfluency"];
    full.extend(args);
    parse_args(full)
}

#[test]
fn shorthand_dash_i_is_run() {
    let cmd = parse(&["-i", "a.txt"]).unwrap();
    match cmd {
        VdCommand::Run(r) => assert_eq!(r.input, PathBuf::from("a.txt")),
        VdCommand::Config(_) => panic!("expected Run"),
    }
}

#[test]
fn run_defaults() {
    let cmd = parse(&["run", "-i", "a.txt"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert!(!r.in_place);
    assert!(!r.overwrite);
    assert!(!r.dry_run);
    assert!(r.language.is_none());
    assert!(r.mode.is_none());
    assert!(r.remove_fillers.is_none());
}

#[test]
fn output_and_dir_conflict() {
    let err = parse(&["run", "-i", "a.txt", "-o", "b.txt", "-d", "out/"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn language_ru() {
    let cmd = parse(&["run", "-i", "a.txt", "-l", "ru"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.language, Some(Language::Ru));
}

#[test]
fn mode_flag_parses() {
    let cmd = parse(&["run", "-i", "a.txt", "-m", "aggressive"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.mode, Some(Mode::Aggressive));
}

#[test]
fn invalid_mode_rejected() {
    let err = parse(&["run", "-i", "a.txt", "-m", "extreme"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn no_fillers_flag_sets_override() {
    let cmd = parse(&["run", "-i", "a.txt", "--no-fillers"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.remove_fillers, Some(false));
}

#[test]
fn json_requires_dry_run() {
    let err = parse(&["run", "-i", "a.txt", "--json"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}
