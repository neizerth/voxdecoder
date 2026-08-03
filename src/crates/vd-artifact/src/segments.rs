//! Speaker-aware segment access for diarized JSON/JSONL artifacts.
//!
//! `TextSpan` (see `text_spans.rs`) deliberately exposes only mutable text —
//! that is the mechanism that makes every `vd-fix-*` CLI's "never touches
//! timestamps/speakers/structure" claim structurally true. Duplicate-speech
//! removal (`vd-fix-overlap`, ADR 0012) genuinely needs more: it must read
//! speaker + timestamp + text *together* to detect a duplicate turn, and
//! then delete that turn's whole record, not just edit its text.
//!
//! This module is the one sanctioned, narrow exception: a **read-only**
//! [`Segment`] snapshot bundling speaker/timing/text for JSON/JSONL
//! array-of-turn-object shapes (matching `vd-pipeline`'s `MeetingTurn`:
//! `{ speaker, start_sec, end_sec, text }`), plus [`remove_segments`], which
//! deletes whole matched array elements. Only JSON/JSONL are in scope —
//! `Txt`/`Md` are single-span with no notion of multiple turns, and
//! `Srt`/`Vtt` carry timing but no structural speaker field, so overlap
//! detection cannot be verified as cross-speaker for them.
//!
//! `SegmentId` is a separate numbering scheme from `TextSpan`'s `SpanId` —
//! segment discovery only visits array-element objects with a recognized
//! text key, a different (usually smaller) count than every text span
//! `apply_to_text_spans` finds. Do not mix the two id spaces.

use serde_json::{Map, Value};

use super::formats::is_text_key;
use super::load::Artifact;

const SPEAKER_KEYS: &[&str] = &["speaker", "speaker_id", "speaker_label"];
const START_KEYS: &[&str] = &["start_sec", "start_time", "start"];
const END_KEYS: &[&str] = &["end_sec", "end_time", "end"];

/// Identity for a [`Segment`], stable within one `collect_segments` /
/// `remove_segments` pair of calls on the same artifact. Not comparable to
/// `SpanId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SegmentId(pub u32);

/// Read-only snapshot of one speaker turn.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub id: SegmentId,
    pub speaker: Option<String>,
    pub start_sec: Option<f64>,
    pub end_sec: Option<f64>,
    pub text: String,
}

/// Collects every JSON/JSONL array element shaped like a speaker turn (an
/// object with a recognized text key). Empty for `Txt`/`Md`/`Srt`/`Vtt`.
pub fn collect_segments(artifact: &Artifact) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut index = 0u32;
    match artifact {
        Artifact::Json(b) => collect_json_segments(&b.value, &mut index, &mut out),
        Artifact::Jsonl(b) => {
            for line in &b.lines {
                collect_json_segments(line, &mut index, &mut out);
            }
        }
        Artifact::Txt(_) | Artifact::Md(_) | Artifact::Srt(_) | Artifact::Vtt(_) => {}
    }
    out
}

/// Deletes the array elements matching `ids` (as produced by a prior
/// `collect_segments` call on this same artifact). Returns how many were
/// removed. No-op (returns `0`) for `Txt`/`Md`/`Srt`/`Vtt`.
pub fn remove_segments(artifact: &mut Artifact, ids: &[SegmentId]) -> usize {
    if ids.is_empty() {
        return 0;
    }
    let mut index = 0u32;
    let mut removed = 0usize;
    match artifact {
        Artifact::Json(b) => remove_json_segments(&mut b.value, &mut index, ids, &mut removed),
        Artifact::Jsonl(b) => {
            // A JSONL line can itself be a bare segment object (no wrapping
            // array), which no `&mut Value` can delete from within itself —
            // only the parent `Vec<Value>` can drop the whole line.
            let mut kept = Vec::with_capacity(b.lines.len());
            for mut line in std::mem::take(&mut b.lines) {
                let is_top_level_segment =
                    matches!(&line, Value::Object(map) if segment_text(map).is_some());
                if is_top_level_segment {
                    let id = SegmentId(index);
                    index += 1;
                    if ids.contains(&id) {
                        removed += 1;
                        continue;
                    }
                } else {
                    remove_json_segments(&mut line, &mut index, ids, &mut removed);
                }
                kept.push(line);
            }
            b.lines = kept;
        }
        Artifact::Txt(_) | Artifact::Md(_) | Artifact::Srt(_) | Artifact::Vtt(_) => {}
    }
    removed
}

/// Overwrites one segment's text in place, identified by the id a prior
/// `collect_segments` call produced on this same artifact.
///
/// Every other field on that segment (and every other segment) is
/// untouched. Returns `false` if no segment with that id exists. No-op
/// (returns `false`) for `Txt`/`Md`/`Srt`/`Vtt`.
pub fn set_segment_text(artifact: &mut Artifact, id: SegmentId, text: &str) -> bool {
    let mut index = 0u32;
    let mut applied = false;
    match artifact {
        Artifact::Json(b) => {
            set_json_segment_text(&mut b.value, &mut index, id, text, &mut applied)
        }
        Artifact::Jsonl(b) => {
            for line in &mut b.lines {
                if applied {
                    break;
                }
                set_json_segment_text(line, &mut index, id, text, &mut applied);
            }
        }
        Artifact::Txt(_) | Artifact::Md(_) | Artifact::Srt(_) | Artifact::Vtt(_) => {}
    }
    applied
}

fn set_json_segment_text(
    value: &mut Value,
    index: &mut u32,
    id: SegmentId,
    text: &str,
    applied: &mut bool,
) {
    if *applied {
        return;
    }
    match value {
        Value::Object(map) => {
            if let Some(key) = text_key(map) {
                if SegmentId(*index) == id {
                    map.insert(key, Value::String(text.to_string()));
                    *applied = true;
                }
                *index += 1;
                return;
            }
            for v in map.values_mut() {
                set_json_segment_text(v, index, id, text, applied);
                if *applied {
                    return;
                }
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                set_json_segment_text(v, index, id, text, applied);
                if *applied {
                    return;
                }
            }
        }
        _ => {}
    }
}

fn text_key(map: &Map<String, Value>) -> Option<String> {
    map.keys().find(|k| is_text_key(k)).cloned()
}

fn segment_text(map: &Map<String, Value>) -> Option<String> {
    let key = text_key(map)?;
    map.get(&key)?.as_str().map(str::to_string)
}

fn find_string(map: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    map.iter().find_map(|(k, v)| {
        if keys.contains(&k.to_ascii_lowercase().as_str()) {
            v.as_str().map(str::to_string)
        } else {
            None
        }
    })
}

fn find_f64(map: &Map<String, Value>, keys: &[&str]) -> Option<f64> {
    map.iter().find_map(|(k, v)| {
        if keys.contains(&k.to_ascii_lowercase().as_str()) {
            v.as_f64()
        } else {
            None
        }
    })
}

fn collect_json_segments(value: &Value, index: &mut u32, out: &mut Vec<Segment>) {
    match value {
        Value::Object(map) => {
            if let Some(text) = segment_text(map) {
                out.push(Segment {
                    id: SegmentId(*index),
                    speaker: find_string(map, SPEAKER_KEYS),
                    start_sec: find_f64(map, START_KEYS),
                    end_sec: find_f64(map, END_KEYS),
                    text,
                });
                *index += 1;
                return; // don't also recurse into a matched segment's own fields
            }
            for v in map.values() {
                collect_json_segments(v, index, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_json_segments(v, index, out);
            }
        }
        _ => {}
    }
}

fn remove_json_segments(
    value: &mut Value,
    index: &mut u32,
    ids: &[SegmentId],
    removed: &mut usize,
) {
    match value {
        Value::Array(arr) => {
            let mut i = 0;
            while i < arr.len() {
                let is_segment =
                    matches!(&arr[i], Value::Object(map) if segment_text(map).is_some());
                if is_segment {
                    let id = SegmentId(*index);
                    *index += 1;
                    if ids.contains(&id) {
                        arr.remove(i);
                        *removed += 1;
                        continue;
                    }
                } else {
                    remove_json_segments(&mut arr[i], index, ids, removed);
                }
                i += 1;
            }
        }
        Value::Object(map) => {
            for v in map.values_mut() {
                remove_json_segments(v, index, ids, removed);
            }
        }
        _ => {}
    }
}
