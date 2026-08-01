//! Output path resolution: `-o` / `-d` / `--in-place` / `.fixed.` / `--overwrite`.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use vd_fix_casing::output::{resolve_output_path, OutputPathError, OutputPathRequest};
use vd_fix_casing::types::ArtifactType;

fn req(
    input: PathBuf,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    in_place: bool,
    overwrite: bool,
) -> OutputPathRequest {
    OutputPathRequest {
        input,
        output,
        output_dir,
        in_place,
        artifact_type: ArtifactType::Txt,
        overwrite,
    }
}

#[test]
fn default_fixed_next_to_input() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "x").unwrap();
    let paths = resolve_output_path(req(input.clone(), None, None, false, false)).unwrap();
    assert_eq!(paths.main, dir.path().join("meeting.fixed.txt"));
    assert!(!paths.in_place);
}

#[test]
fn output_dir_uses_fixed_name() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.srt");
    let out = dir.path().join("cleaned");
    fs::create_dir(&out).unwrap();
    let paths = resolve_output_path(OutputPathRequest {
        input,
        output: None,
        output_dir: Some(out.clone()),
        in_place: false,
        artifact_type: ArtifactType::Srt,
        overwrite: false,
    })
    .unwrap();
    assert_eq!(paths.main, out.join("meeting.fixed.srt"));
}

#[test]
fn in_place() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.txt");
    fs::write(&input, "x").unwrap();
    let paths = resolve_output_path(req(input.clone(), None, None, true, false)).unwrap();
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
    let err = resolve_output_path(req(input, None, None, false, false)).unwrap_err();
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
    ))
    .unwrap_err();
    assert!(matches!(err, OutputPathError::ConflictingTargets));
}
