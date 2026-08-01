//! CLI argv parse.

use vd_diarize::cli::{parse_args, Command};

#[test]
fn shorthand_inserts_run() {
    let cmd = parse_args(["vd-diarize", "-i", "a.wav"]).unwrap();
    match cmd {
        Command::Run(args) => {
            assert_eq!(args.input, std::path::PathBuf::from("a.wav"));
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn dry_run_json_ok() {
    let cmd = parse_args([
        "vd-diarize",
        "run",
        "-i",
        "a.wav",
        "--dry-run",
        "--json",
    ])
    .unwrap();
    match cmd {
        Command::Run(args) => {
            assert!(args.dry_run);
            assert!(args.json);
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn json_without_dry_run_fails() {
    let err = parse_args(["vd-diarize", "run", "-i", "a.wav", "--json"]).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}
