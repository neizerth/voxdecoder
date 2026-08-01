//! Text spans: only transcript text is mutable; structure untouched.

use vd_fix_casing::artifact::{apply_to_text_spans, load_from_str, Artifact};
use vd_fix_casing::types::{ArtifactType, TextSpan};

#[test]
fn srt_preserves_timing() {
    let raw = "1\n00:00:01,000 --> 00:00:02,000\nhello world\n";
    let mut art = load_from_str(ArtifactType::Srt, raw).unwrap();
    apply_to_text_spans(&mut art, |span: TextSpan<'_>| -> Result<(), ()> {
        *span.text = "Hello world.".to_string();
        Ok(())
    })
    .unwrap();
    let Artifact::Srt(body) = &art else {
        panic!("expected srt");
    };
    assert_eq!(body.cues[0].timing, "00:00:01,000 --> 00:00:02,000");
    assert_eq!(body.cues[0].index, "1");
    assert_eq!(body.cues[0].text, "Hello world.");
}

#[test]
fn json_preserves_id_and_start() {
    let raw = r#"{"id":"seg-1","start":1.5,"text":"hello"}"#;
    let mut art = load_from_str(ArtifactType::Json, raw).unwrap();
    apply_to_text_spans(&mut art, |span: TextSpan<'_>| -> Result<(), ()> {
        *span.text = "Hello.".to_string();
        Ok(())
    })
    .unwrap();
    let Artifact::Json(body) = &art else {
        panic!("expected json");
    };
    assert_eq!(body.value["id"], "seg-1");
    assert_eq!(body.value["start"], 1.5);
    assert_eq!(body.value["text"], "Hello.");
}

#[test]
fn json_finds_utterance_and_caption_case_insensitive() {
    let raw = r#"{"ID":"1","Utterance":"one","segments":[{"Caption":"two","speaker":"A"}]}"#;
    let mut art = load_from_str(ArtifactType::Json, raw).unwrap();
    let mut seen = Vec::new();
    apply_to_text_spans(&mut art, |span: TextSpan<'_>| -> Result<(), ()> {
        seen.push(span.text.clone());
        *span.text = format!("{}!", span.text);
        Ok(())
    })
    .unwrap();
    assert_eq!(seen, vec!["one".to_string(), "two".to_string()]);
    let Artifact::Json(body) = &art else {
        panic!("expected json");
    };
    assert_eq!(body.value["ID"], "1");
    assert_eq!(body.value["Utterance"], "one!");
    assert_eq!(body.value["segments"][0]["speaker"], "A");
    assert_eq!(body.value["segments"][0]["Caption"], "two!");
}

#[test]
fn vtt_preserves_cue_id_and_timing() {
    let raw = "WEBVTT\n\ncue-42\n00:00:01.000 --> 00:00:02.000\nhello there\n";
    let mut art = load_from_str(ArtifactType::Vtt, raw).unwrap();
    apply_to_text_spans(&mut art, |span: TextSpan<'_>| -> Result<(), ()> {
        *span.text = "Hello there.".to_string();
        Ok(())
    })
    .unwrap();
    let Artifact::Vtt(body) = &art else {
        panic!("expected vtt");
    };
    let Some(vd_fix_casing::artifact::VttBlock::Cue { id, timing, text }) = body
        .blocks
        .iter()
        .find(|b| matches!(b, vd_fix_casing::artifact::VttBlock::Cue { .. }))
    else {
        panic!("expected cue");
    };
    assert_eq!(id.as_deref(), Some("cue-42"));
    assert_eq!(timing, "00:00:01.000 --> 00:00:02.000");
    assert_eq!(text, "Hello there.");
}
