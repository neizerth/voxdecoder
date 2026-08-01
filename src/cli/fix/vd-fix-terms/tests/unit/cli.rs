//! CLI parsing and validation.

use std::path::PathBuf;

use vd_fix_terms::cli::{parse_args, CliError, Command as VdCommand};
use vd_fix_terms::types::Language;

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-fix-terms"];
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
fn run_defaults_shipping_on() {
    let cmd = parse(&["run", "-i", "a.txt"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert!(!r.in_place);
    assert!(!r.overwrite);
    assert!(!r.dry_run);
    assert!(r.language.is_none());
    assert!(r.terms.is_empty());
    assert!(r.shipping);
}

#[test]
fn terms_repeatable() {
    let cmd = parse(&[
        "run", "-i", "a.txt", "--terms", "./a.yaml", "--terms", "./b.yaml",
    ])
    .unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert_eq!(
        r.terms,
        vec![PathBuf::from("./a.yaml"), PathBuf::from("./b.yaml")]
    );
}

#[test]
fn no_shipping_lexicon() {
    let cmd = parse(&["run", "-i", "a.txt", "--no-shipping-lexicon"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert!(!r.shipping);
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
