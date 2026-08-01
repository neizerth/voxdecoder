//! SpeakerTimeline serialize / validate.

use std::path::PathBuf;

use tempfile::TempDir;
use vd_diarize::{
    AudioRef, BackendInfo, Segment, SpeakerId, SpeakerTimeline,
};

#[test]
fn roundtrip_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("t.diarization.json");
    let t = SpeakerTimeline {
        version: 1,
        audio: AudioRef {
            path: PathBuf::from("a.wav"),
        },
        speakers: vec![SpeakerId { id: "S0".into() }],
        segments: vec![Segment {
            speaker: "S0".into(),
            start: 0.0,
            end: 1.5,
            confidence: Some(0.9),
        }],
        overlaps: Vec::new(),
        embeddings: None,
        speech_regions: Vec::new(),
        backend: BackendInfo {
            provider: "stub".into(),
            model: "deterministic-v1".into(),
            version: Some("1".into()),
            device: Some("cpu".into()),
        },
    };
    t.write_json(&path).unwrap();
    let back = SpeakerTimeline::read_json(&path).unwrap();
    assert_eq!(back.speakers[0].id, "S0");
    assert_eq!(back.segments.len(), 1);
}

#[test]
fn reject_bad_segment() {
    let t = SpeakerTimeline {
        version: 1,
        audio: AudioRef {
            path: PathBuf::from("a.wav"),
        },
        speakers: vec![],
        segments: vec![Segment {
            speaker: "S0".into(),
            start: 2.0,
            end: 1.0,
            confidence: None,
        }],
        overlaps: Vec::new(),
        embeddings: None,
        speech_regions: Vec::new(),
        backend: BackendInfo {
            provider: "stub".into(),
            model: "x".into(),
            version: None,
            device: None,
        },
    };
    assert!(t.validate().is_err());
}
