//! ArtifactRef::parse — dotted artifact ids vs filesystem paths.

use std::path::PathBuf;

use vd_pipeline::ArtifactRef;

#[test]
fn meeting_stage_ids_are_artifact_ids() {
    for raw in [
        "igor.transcript",
        "vladimir.cased",
        "alice.asr",
        "bob.text",
        "room.prepared",
        "timeline",
        "assets",
        "meeting",
    ] {
        assert_eq!(
            ArtifactRef::parse(raw),
            ArtifactRef::Id(raw.into()),
            "{raw} must be an artifact id"
        );
    }
}

#[test]
fn known_media_extensions_are_paths() {
    for raw in [
        "meeting.wav",
        "alice.mp3",
        "room.m4a",
        "clip.ogg",
        "notes.txt",
    ] {
        assert_eq!(
            ArtifactRef::parse(raw),
            ArtifactRef::Path(PathBuf::from(raw)),
            "{raw} must be a filesystem path"
        );
    }
}

#[test]
fn separators_force_path() {
    assert_eq!(
        ArtifactRef::parse("/work/igor.transcript"),
        ArtifactRef::Path(PathBuf::from("/work/igor.transcript"))
    );
    assert_eq!(
        ArtifactRef::parse("out/alice.wav"),
        ArtifactRef::Path(PathBuf::from("out/alice.wav"))
    );
}

#[test]
fn wildcard_stays_id() {
    assert_eq!(
        ArtifactRef::parse("branch/*"),
        ArtifactRef::Id("branch/*".into())
    );
}
