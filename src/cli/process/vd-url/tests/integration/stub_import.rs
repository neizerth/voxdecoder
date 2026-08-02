use std::fs;

use tempfile::tempdir;
use vd_url::{resolve, SubtitlePolicy, UrlImportRequest};

#[test]
fn stub_full_import() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("out");
    let result = resolve(&UrlImportRequest {
        url: "https://example.com/stub".into(),
        provider: Some("stub".into()),
        subtitles: SubtitlePolicy::Prefer,
        metadata_only: false,
        output_dir: out.clone(),
        overwrite: true,
    })
    .unwrap();

    assert!(result.audio.as_ref().unwrap().path.is_file());
    assert!(result.metadata.path.is_file());
    assert!(result.subtitle.as_ref().unwrap().path.is_file());

    let meta = fs::read_to_string(&result.metadata.path).unwrap();
    assert!(meta.contains("provider: stub") || meta.contains("provider:stub"));
}

#[test]
fn stub_metadata_only() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("meta");
    let result = resolve(&UrlImportRequest {
        url: "https://example.com/stub".into(),
        provider: Some("stub".into()),
        subtitles: SubtitlePolicy::Ignore,
        metadata_only: true,
        output_dir: out,
        overwrite: true,
    })
    .unwrap();
    assert!(result.audio.is_none());
    assert!(result.metadata.path.is_file());
}

#[test]
fn cli_validate_and_providers() {
    use assert_cmd::Command;
    Command::cargo_bin("vd-url")
        .unwrap()
        .args([
            "validate",
            "-i",
            "https://youtu.be/abcdefghijk",
            "--subtitles",
            "prefer",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("URL valid"));

    Command::cargo_bin("vd-url")
        .unwrap()
        .args(["providers"])
        .assert()
        .success()
        .stdout(predicates::str::contains("youtube"));
}

#[test]
fn cli_stub_run_json() {
    use assert_cmd::Command;
    let dir = tempdir().unwrap();
    let out = dir.path().join("cli-out");
    Command::cargo_bin("vd-url")
        .unwrap()
        .args([
            "run",
            "-i",
            "https://example.com/x",
            "--provider",
            "stub",
            "--output-dir",
            out.to_str().unwrap(),
            "--overwrite",
            "-o",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"kind\": \"audio\""));
}
