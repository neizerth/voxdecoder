//! Meeting document parse.

use std::path::PathBuf;

use tempfile::TempDir;
use vd_meeting::model::load_meeting_file;
use vd_meeting::{DiarizationEnabled, InputRole};

#[test]
fn yaml_document() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("m.yaml");
    std::fs::write(
        &path,
        r#"
version: 1
working_dir: .
inputs:
  - role: merged
    path: meeting.wav
  - role: participant
    participant: alice
    path: alice.wav
meeting:
  participants:
    known:
      - name: Alice
        constraints:
          gender: female
  diarization:
    enabled: auto
  alignment:
    mode: longest
"#,
    )
    .unwrap();
    let (req, _) = load_meeting_file(&path).unwrap();
    assert_eq!(req.inputs.len(), 2);
    assert_eq!(req.inputs[0].role, InputRole::Room);
    assert_eq!(req.inputs[1].participant.as_deref(), Some("alice"));
    assert_eq!(req.meeting.diarization.enabled, DiarizationEnabled::Auto);
    assert!(req.working_dir == Some(PathBuf::from(".")));
}

#[test]
fn yaml_enabled_bool_true() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("m.yaml");
    std::fs::write(
        &path,
        r#"
version: 1
inputs:
  - role: room
    path: meeting.wav
meeting:
  diarization:
    enabled: true
"#,
    )
    .unwrap();
    let (req, _) = load_meeting_file(&path).unwrap();
    assert_eq!(req.meeting.diarization.enabled, DiarizationEnabled::True);
}

#[test]
fn json_enabled_bool_true() {
    let raw = r#"{"inputs":[{"role":"room","path":"m.wav"}],"meeting":{"diarization":{"enabled":true}}}"#;
    let req: vd_meeting::MeetingRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.meeting.diarization.enabled, DiarizationEnabled::True);
}
