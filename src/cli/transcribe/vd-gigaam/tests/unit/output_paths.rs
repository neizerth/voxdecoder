//! Output path resolution: `-o` XOR `-d`, segments sidecar, overwrite checks.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use vd_gigaam::config::resolve::OutputFormat;
use vd_gigaam::output::{resolve_output_paths, OutputPathError, OutputPathRequest};

fn req(
    input: &str,
    output: Option<&str>,
    output_dir: Option<&str>,
    format: OutputFormat,
    segments: bool,
    overwrite: bool,
) -> OutputPathRequest {
    OutputPathRequest {
        input: PathBuf::from(input),
        output: output.map(PathBuf::from),
        output_dir: output_dir.map(PathBuf::from),
        format,
        segments,
        overwrite,
    }
}

#[test]
fn default_output_next_to_input() {
    let paths = resolve_output_paths(req(
        "/path/meeting.ogg",
        None,
        None,
        OutputFormat::Txt,
        false,
        false,
    ))
    .unwrap();
    assert_eq!(paths.main, PathBuf::from("/path/meeting.txt"));
    assert_eq!(paths.segments, None);
}

#[test]
fn format_extension() {
    for (format, ext) in [
        (OutputFormat::Txt, "txt"),
        (OutputFormat::Json, "json"),
        (OutputFormat::Srt, "srt"),
        (OutputFormat::Vtt, "vtt"),
    ] {
        let paths =
            resolve_output_paths(req("/a/b/c.mp3", None, None, format, false, false)).unwrap();
        assert_eq!(paths.main, PathBuf::from(format!("/a/b/c.{ext}")));
    }
}

#[test]
fn explicit_output_file() {
    let paths = resolve_output_paths(req(
        "/path/meeting.ogg",
        Some("./out/result.txt"),
        None,
        OutputFormat::Txt,
        true,
        false,
    ))
    .unwrap();
    assert_eq!(paths.main, PathBuf::from("./out/result.txt"));
    assert_eq!(
        paths.segments,
        Some(PathBuf::from("./out/result.segments.json"))
    );
}

#[test]
fn output_dir_uses_input_stem() {
    let paths = resolve_output_paths(req(
        "/path/meeting.ogg",
        None,
        Some("./transcripts/"),
        OutputFormat::Txt,
        true,
        false,
    ))
    .unwrap();
    assert_eq!(paths.main, PathBuf::from("./transcripts/meeting.txt"));
    assert_eq!(
        paths.segments,
        Some(PathBuf::from("./transcripts/meeting.segments.json"))
    );
}

#[test]
fn segments_follow_main_output_not_input() {
    let paths = resolve_output_paths(req(
        "/other/input.wav",
        Some("out/foo.txt"),
        None,
        OutputFormat::Srt,
        true,
        false,
    ))
    .unwrap();
    assert_eq!(paths.segments, Some(PathBuf::from("out/foo.segments.json")));
}

#[test]
fn rejects_both_output_and_dir() {
    let err = resolve_output_paths(req(
        "a.wav",
        Some("o.txt"),
        Some("dir"),
        OutputFormat::Txt,
        false,
        false,
    ))
    .unwrap_err();
    assert!(matches!(err, OutputPathError::ConflictingTargets));
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn existing_output_without_overwrite_is_exit_2() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("meeting.txt");
    fs::write(&out, "exists").unwrap();

    let err = resolve_output_paths(OutputPathRequest {
        input: dir.path().join("meeting.ogg"),
        output: Some(out.clone()),
        output_dir: None,
        format: OutputFormat::Txt,
        segments: false,
        overwrite: false,
    })
    .unwrap_err();
    assert!(matches!(err, OutputPathError::AlreadyExists(_)));
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn overwrite_allows_existing() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("meeting.txt");
    fs::write(&out, "exists").unwrap();

    let paths = resolve_output_paths(OutputPathRequest {
        input: dir.path().join("meeting.ogg"),
        output: Some(out.clone()),
        output_dir: None,
        format: OutputFormat::Txt,
        segments: false,
        overwrite: true,
    })
    .unwrap();
    assert_eq!(paths.main, out);
}

#[test]
fn existing_segments_sidecar_blocks_without_overwrite() {
    let dir = TempDir::new().unwrap();
    let main = dir.path().join("meeting.txt");
    let seg = dir.path().join("meeting.segments.json");
    fs::write(&seg, "{}").unwrap();

    let err = resolve_output_paths(OutputPathRequest {
        input: dir.path().join("meeting.ogg"),
        output: Some(main),
        output_dir: None,
        format: OutputFormat::Txt,
        segments: true,
        overwrite: false,
    })
    .unwrap_err();
    assert!(matches!(err, OutputPathError::AlreadyExists(_)));
}
