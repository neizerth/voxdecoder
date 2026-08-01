//! CLI parsing and validation.

use std::path::PathBuf;

use vd_fix_asr::cli::{parse_args, CliError, Command as VdCommand};
use vd_fix_asr::types::Language;

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-fix-asr"];
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
    assert!(r.context.is_empty());
    assert!(r.context_neighbors.is_none());
}

#[test]
fn context_repeatable() {
    let cmd = parse(&[
        "run",
        "-i",
        "a.txt",
        "--context",
        "./docs",
        "--context",
        "./glossary.yaml",
    ])
    .unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(
        r.context,
        vec![PathBuf::from("./docs"), PathBuf::from("./glossary.yaml")]
    );
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
fn json_requires_dry_run() {
    let err = parse(&["run", "-i", "a.txt", "--json"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}
