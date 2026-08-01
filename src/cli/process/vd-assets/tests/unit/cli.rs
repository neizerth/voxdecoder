//! CLI parsing.

use std::path::PathBuf;

use vd_assets::cli::{parse_args, CliError, Command as VdCommand};

fn parse(args: &[&str]) -> Result<VdCommand, CliError> {
    let mut full = vec!["vd-assets"];
    full.extend(args);
    parse_args(full)
}

#[test]
fn shorthand_dash_i_is_run() {
    let cmd = parse(&["-i", "./docs", "-o", "./out"]).unwrap();
    match cmd {
        VdCommand::Run(r) => {
            assert_eq!(r.input, vec![PathBuf::from("./docs")]);
            assert_eq!(r.output, PathBuf::from("./out"));
        }
        VdCommand::Config(_) => panic!("expected Run"),
    }
}

#[test]
fn output_defaults_to_dot_voxdecoder() {
    let cmd = parse(&["-i", "./docs"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert!(r.output.ends_with(".voxdecoder"));
}

#[test]
fn json_requires_dry_run() {
    let err = parse(&["run", "-i", "a", "-o", "b", "--json"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn ocr_flag() {
    let cmd = parse(&["run", "-i", "a.pdf", "-o", "out", "--ocr"]).unwrap();
    let VdCommand::Run(r) = cmd else {
        panic!("expected Run");
    };
    assert!(r.ocr);
}
