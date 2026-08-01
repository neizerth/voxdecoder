//! Load → identity spans → write; type preserved.

use std::fs;

use tempfile::TempDir;
use vd_artifact::{apply_to_text_spans, load, load_from_str, write, ArtifactType, TextSpan};

#[test]
fn txt_roundtrip_identity() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.txt");
    fs::write(&path, "Hello world.").unwrap();
    let mut art = load(&path).unwrap();
    apply_to_text_spans(&mut art, |_span: TextSpan<'_>| Ok::<(), ()>(())).unwrap();
    let out = dir.path().join("a.fixed.txt");
    write(&art, &out).unwrap();
    assert_eq!(fs::read_to_string(&out).unwrap(), "Hello world.");
}

#[test]
fn srt_roundtrip_preserves_structure() {
    let raw = "1\n00:00:01,000 --> 00:00:02,000\nhello\n";
    let art = load_from_str(ArtifactType::Srt, raw).unwrap();
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("a.srt");
    write(&art, &out).unwrap();
    let back = load(&out).unwrap();
    assert_eq!(back.artifact_type(), ArtifactType::Srt);
}
