//! Library `diarize` with stub backend.

use std::collections::BTreeMap;
use std::fs;

use tempfile::TempDir;
use vd_diarize::progress::ProgressMode;
use vd_diarize::{diarize, BackendSpec, DiarizeRequest, SpeakerTimeline};

#[test]
fn stub_writes_timeline() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("meeting.wav");
    fs::write(&input, vec![0u8; 32_000]).unwrap();
    let output = dir.path().join("out.diarization.json");

    let req = DiarizeRequest {
        input,
        output: Some(output.clone()),
        backend: BackendSpec::new("stub", None),
        device: Some("cpu".into()),
        options: BTreeMap::default(),
    };
    let out = diarize(&req, ProgressMode::None, true).unwrap();
    assert_eq!(out.output, output);
    let t = SpeakerTimeline::read_json(&output).unwrap();
    assert_eq!(t.backend.provider, "stub");
    assert_eq!(t.speakers.len(), 2);
    assert!(!t.segments.is_empty());
}
