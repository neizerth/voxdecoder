//! CLI parsing, validation, and exit-code contracts from `cli.md`.

use std::path::PathBuf;

use vd_fix_casing::cli::{parse_args, CliError, Command as VdCommand};
use vd_fix_casing::types::Language;

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-fix-casing"];
    full.extend(args);
    parse_args(full)
}

#[test]
fn shorthand_dash_i_is_run() {
    let cmd = parse(&["-i", "a.txt"]).unwrap();
    match cmd {
        VdCommand::Run(r) => assert_eq!(r.input, PathBuf::from("a.txt")),
        VdCommand::Config(_)
        | VdCommand::Install(_)
        | VdCommand::Remove(_)
        | VdCommand::List(_)
        | VdCommand::Info(_) => panic!("expected Run"),
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
}

#[test]
fn output_and_dir_conflict() {
    let err = parse(&["run", "-i", "a.txt", "-o", "b.txt", "-d", "out/"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn output_and_in_place_conflict() {
    let err = parse(&["run", "-i", "a.txt", "-o", "b.txt", "--in-place"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn language_ru_en() {
    let cmd = parse(&["run", "-i", "a.txt", "-l", "en"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.language, Some(Language::En));
}

#[test]
fn json_requires_dry_run() {
    let err = parse(&["run", "-i", "a.txt", "--json"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}
