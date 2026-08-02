//! CLI parsing, validation, and exit-code contracts from `cli.md`.

use std::path::PathBuf;

use vd_fix_layout::cli::{parse_args, CliError, Command as VdCommand};
use vd_fix_layout::types::{Language, ParagraphDensity};

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-fix-layout"];
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
    assert!(r.density.is_none());
}

#[test]
fn language_auto() {
    let cmd = parse(&["run", "-i", "a.txt", "-l", "auto"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.language, Some(Language::Auto));
}

#[test]
fn density_relaxed() {
    let cmd = parse(&["run", "-i", "a.txt", "--density", "relaxed"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(r.density, Some(ParagraphDensity::Relaxed));
}

#[test]
fn reject_de_language() {
    let err = parse(&["run", "-i", "a.txt", "-l", "de"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn output_and_dir_conflict() {
    let err = parse(&["run", "-i", "a.txt", "-o", "b.txt", "-d", "out/"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn json_requires_dry_run() {
    let err = parse(&["run", "-i", "a.txt", "--json"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn timemap_and_no_timemap_conflict() {
    let err = parse(&[
        "run",
        "-i",
        "a.txt",
        "--timemap",
        "x.json",
        "--no-timemap",
    ])
    .unwrap_err();
    assert_eq!(err.exit_code(), 2);
}
