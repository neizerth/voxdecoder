//! Speaker-aware segments: read-only snapshot + narrow structural removal
//! for JSON/JSONL turn arrays (ADR 0012 `vd-fix-overlap`).

use vd_artifact::{
    collect_segments, load_from_str, remove_segments, set_segment_text, Artifact, ArtifactType,
    SegmentId,
};

const MEETING_JSON: &str = r#"{
  "version": 1,
  "turns": [
    {"speaker": "A", "start_sec": 1.0, "end_sec": 3.0, "text": "Let's deploy tomorrow."},
    {"speaker": "B", "start_sec": 1.2, "end_sec": 3.2, "text": "let's deploy tomorrow"},
    {"speaker": "A", "start_sec": 3.2, "end_sec": 4.0, "text": "Sounds good."}
  ]
}"#;

#[test]
fn collects_speaker_start_end_text_together() {
    let art = load_from_str(ArtifactType::Json, MEETING_JSON).unwrap();
    let segments = collect_segments(&art);
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].speaker.as_deref(), Some("A"));
    assert_eq!(segments[0].start_sec, Some(1.0));
    assert_eq!(segments[0].end_sec, Some(3.0));
    assert_eq!(segments[0].text, "Let's deploy tomorrow.");
    assert_eq!(segments[1].speaker.as_deref(), Some("B"));
}

#[test]
fn collects_across_jsonl_lines() {
    let raw = "{\"speaker\":\"A\",\"start_sec\":0.0,\"end_sec\":1.0,\"text\":\"hi\"}\n{\"speaker\":\"B\",\"start_sec\":1.0,\"end_sec\":2.0,\"text\":\"hey\"}\n";
    let art = load_from_str(ArtifactType::Jsonl, raw).unwrap();
    let segments = collect_segments(&art);
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[1].speaker.as_deref(), Some("B"));
}

#[test]
fn empty_for_txt_md_srt_vtt() {
    let txt = load_from_str(ArtifactType::Txt, "hello").unwrap();
    assert!(collect_segments(&txt).is_empty());
    let md = load_from_str(ArtifactType::Md, "# hi").unwrap();
    assert!(collect_segments(&md).is_empty());
    let srt = load_from_str(ArtifactType::Srt, "1\n00:00:01,000 --> 00:00:02,000\nhi\n").unwrap();
    assert!(collect_segments(&srt).is_empty());
    let vtt = load_from_str(
        ArtifactType::Vtt,
        "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nhi\n",
    )
    .unwrap();
    assert!(collect_segments(&vtt).is_empty());
}

#[test]
fn remove_segments_deletes_matched_turns_only() {
    let mut art = load_from_str(ArtifactType::Json, MEETING_JSON).unwrap();
    let segments = collect_segments(&art);
    let drop_id = segments[1].id; // Speaker B's duplicate
    let removed = remove_segments(&mut art, &[drop_id]);
    assert_eq!(removed, 1);

    let remaining = collect_segments(&art);
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].speaker.as_deref(), Some("A"));
    assert_eq!(remaining[0].text, "Let's deploy tomorrow.");
    assert_eq!(remaining[1].speaker.as_deref(), Some("A"));
    assert_eq!(remaining[1].text, "Sounds good.");
}

#[test]
fn remove_segments_preserves_structure_of_surviving_turns() {
    let mut art = load_from_str(ArtifactType::Json, MEETING_JSON).unwrap();
    let segments = collect_segments(&art);
    remove_segments(&mut art, &[segments[1].id]);
    let Artifact::Json(body) = &art else {
        panic!("expected json");
    };
    assert_eq!(body.value["version"], 1);
    assert_eq!(body.value["turns"].as_array().unwrap().len(), 2);
    assert_eq!(body.value["turns"][0]["start_sec"], 1.0);
}

#[test]
fn remove_segments_unknown_id_is_noop() {
    let mut art = load_from_str(ArtifactType::Json, MEETING_JSON).unwrap();
    let removed = remove_segments(&mut art, &[SegmentId(999)]);
    assert_eq!(removed, 0);
    assert_eq!(collect_segments(&art).len(), 3);
}

#[test]
fn remove_segments_empty_ids_is_noop() {
    let mut art = load_from_str(ArtifactType::Json, MEETING_JSON).unwrap();
    let removed = remove_segments(&mut art, &[]);
    assert_eq!(removed, 0);
    assert_eq!(collect_segments(&art).len(), 3);
}

#[test]
fn remove_segments_is_noop_for_txt_srt() {
    let mut txt = load_from_str(ArtifactType::Txt, "hello").unwrap();
    assert_eq!(remove_segments(&mut txt, &[SegmentId(0)]), 0);

    let mut srt =
        load_from_str(ArtifactType::Srt, "1\n00:00:01,000 --> 00:00:02,000\nhi\n").unwrap();
    assert_eq!(remove_segments(&mut srt, &[SegmentId(0)]), 0);
}

#[test]
fn set_segment_text_rewrites_only_the_matched_turn() {
    let mut art = load_from_str(ArtifactType::Json, MEETING_JSON).unwrap();
    let segments = collect_segments(&art);
    let applied = set_segment_text(&mut art, segments[1].id, "trimmed remainder");
    assert!(applied);

    let now = collect_segments(&art);
    assert_eq!(now.len(), 3, "text rewrite must not remove the turn");
    assert_eq!(now[0].text, "Let's deploy tomorrow.");
    assert_eq!(now[1].text, "trimmed remainder");
    assert_eq!(
        now[1].speaker.as_deref(),
        Some("B"),
        "other fields untouched"
    );
    assert_eq!(now[2].text, "Sounds good.");
}

#[test]
fn set_segment_text_unknown_id_returns_false() {
    let mut art = load_from_str(ArtifactType::Json, MEETING_JSON).unwrap();
    assert!(!set_segment_text(&mut art, SegmentId(999), "x"));
    assert_eq!(collect_segments(&art)[0].text, "Let's deploy tomorrow.");
}

#[test]
fn set_segment_text_is_noop_for_txt_srt() {
    let mut txt = load_from_str(ArtifactType::Txt, "hello").unwrap();
    assert!(!set_segment_text(&mut txt, SegmentId(0), "x"));
}

#[test]
fn object_without_recognized_speaker_or_timing_still_collects_text_only() {
    let raw = r#"{"turns": [{"text": "no metadata here"}]}"#;
    let art = load_from_str(ArtifactType::Json, raw).unwrap();
    let segments = collect_segments(&art);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].speaker, None);
    assert_eq!(segments[0].start_sec, None);
    assert_eq!(segments[0].text, "no metadata here");
}
