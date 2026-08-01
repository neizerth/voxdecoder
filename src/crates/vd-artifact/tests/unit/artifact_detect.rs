//! Artifact type detection.

use std::path::Path;

use vd_artifact::detect_type;
use vd_artifact::ArtifactType;

#[test]
fn extensions() {
    assert_eq!(detect_type(Path::new("a.txt")), Some(ArtifactType::Txt));
    assert_eq!(detect_type(Path::new("a.JSON")), Some(ArtifactType::Json));
    assert_eq!(detect_type(Path::new("a.jsonl")), Some(ArtifactType::Jsonl));
    assert_eq!(detect_type(Path::new("a.srt")), Some(ArtifactType::Srt));
    assert_eq!(detect_type(Path::new("a.vtt")), Some(ArtifactType::Vtt));
    assert_eq!(detect_type(Path::new("a.md")), Some(ArtifactType::Md));
    assert_eq!(detect_type(Path::new("a.wav")), None);
}
