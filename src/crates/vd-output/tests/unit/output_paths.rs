//! Output path resolution: naming helpers + `-o` / `-d` / `--in-place` / `--overwrite`.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use vd_output::{
    ensure_writable, fixed_file_name, resolve_output_path, segments_sidecar, stem_ext_file_name,
    OutputPathError, OutputPathRequest,
};

fn req(
    input: PathBuf,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    in_place: bool,
    overwrite: bool,
    default_file_name: String,
) -> OutputPathRequest {
    OutputPathRequest {
        input,
        output,
        output_dir,
        in_place,
        overwrite,
        default_file_name,
    }
}

#[test]
fn default_fixed_next_to_input() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "x").unwrap();
    let name = fixed_file_name(&input, "txt");
    let paths = resolve_output_path(req(input.clone(), None, None, false, false, name)).unwrap();
    assert_eq!(paths.main, dir.path().join("meeting.fixed.txt"));
    assert!(!paths.in_place);
}

#[test]
fn stem_ext_for_transcription() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("voice.mp3");
    fs::write(&input, "x").unwrap();
    let name = stem_ext_file_name(&input, "txt");
    let paths = resolve_output_path(req(input, None, None, false, false, name)).unwrap();
    assert_eq!(paths.main, dir.path().join("voice.txt"));
}

#[test]
fn output_dir_uses_default_name() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.srt");
    let out = dir.path().join("cleaned");
    fs::create_dir(&out).unwrap();
    let paths = resolve_output_path(OutputPathRequest {
        input: input.clone(),
        output: None,
        output_dir: Some(out.clone()),
        in_place: false,
        overwrite: false,
        default_file_name: fixed_file_name(&input, "srt"),
    })
    .unwrap();
    assert_eq!(paths.main, out.join("meeting.fixed.srt"));
}

#[test]
fn in_place() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "x").unwrap();
    let name = fixed_file_name(&input, "txt");
    let paths = resolve_output_path(req(input.clone(), None, None, true, false, name)).unwrap();
    assert_eq!(paths.main, input);
    assert!(paths.in_place);
}

#[test]
fn exists_without_overwrite() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    let fixed = dir.path().join("meeting.fixed.txt");
    fs::write(&input, "x").unwrap();
    fs::write(&fixed, "y").unwrap();
    let name = fixed_file_name(&input, "txt");
    let err = resolve_output_path(req(input, None, None, false, false, name)).unwrap_err();
    assert!(matches!(err, OutputPathError::AlreadyExists(_)));
}

#[test]
fn conflict_flags() {
    let err = resolve_output_path(req(
        PathBuf::from("a.txt"),
        Some(PathBuf::from("b.txt")),
        Some(PathBuf::from("out")),
        false,
        false,
        "a.fixed.txt".into(),
    ))
    .unwrap_err();
    assert!(matches!(err, OutputPathError::ConflictingTargets));
}

#[test]
fn segments_sidecar_and_ensure() {
    let dir = TempDir::new().unwrap();
    let main = dir.path().join("meeting.txt");
    let seg = segments_sidecar(&main);
    assert_eq!(seg, dir.path().join("meeting.segments.json"));
    fs::write(&seg, "{}").unwrap();
    assert!(matches!(
        ensure_writable(&seg, false),
        Err(OutputPathError::AlreadyExists(_))
    ));
    assert!(ensure_writable(&seg, true).is_ok());
}
