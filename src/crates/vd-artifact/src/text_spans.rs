//! Apply a callback to each mutable transcript `TextSpan`. Structure is unreachable.

use super::formats::{is_text_key, VttBlock};
use super::load::Artifact;
use crate::types::{SpanId, TextSpan};

/// Count mutable transcript text spans (for progress).
pub fn count_text_spans(artifact: &Artifact) -> usize {
    match artifact {
        Artifact::Txt(_) | Artifact::Md(_) => 1,
        Artifact::Srt(b) => b.cues.len(),
        Artifact::Vtt(b) => b
            .blocks
            .iter()
            .filter(|b| matches!(b, VttBlock::Cue { .. }))
            .count(),
        Artifact::Json(b) => count_json(&b.value),
        Artifact::Jsonl(b) => b.lines.iter().map(count_json).sum(),
    }
}

fn count_json(value: &serde_json::Value) -> usize {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut n = 0;
            for (k, v) in map {
                if is_text_key(k) {
                    if matches!(v, Value::String(_)) {
                        n += 1;
                    }
                } else {
                    n += count_json(v);
                }
            }
            n
        }
        Value::Array(arr) => arr.iter().map(count_json).sum(),
        _ => 0,
    }
}

/// Snapshot all span texts (read-only) for neighbor windows / analysis.
pub fn collect_span_texts(artifact: &Artifact) -> Vec<String> {
    match artifact {
        Artifact::Txt(b) => vec![b.text.clone()],
        Artifact::Md(b) => vec![b.text.clone()],
        Artifact::Srt(b) => b.cues.iter().map(|c| c.text.clone()).collect(),
        Artifact::Vtt(b) => b
            .blocks
            .iter()
            .filter_map(|block| match block {
                VttBlock::Cue { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect(),
        Artifact::Json(b) => {
            let mut out = Vec::new();
            collect_json_texts(&b.value, &mut out);
            out
        }
        Artifact::Jsonl(b) => {
            let mut out = Vec::new();
            for line in &b.lines {
                collect_json_texts(line, &mut out);
            }
            out
        }
    }
}

fn collect_json_texts(value: &serde_json::Value, out: &mut Vec<String>) {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if is_text_key(k) {
                    if let Value::String(s) = v {
                        out.push(s.clone());
                    }
                } else {
                    collect_json_texts(v, out);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_json_texts(v, out);
            }
        }
        _ => {}
    }
}

/// Visit each mutable transcript text span. Structure fields are not exposed.
pub fn apply_to_text_spans<E, F>(artifact: &mut Artifact, f: F) -> Result<(), E>
where
    F: FnMut(TextSpan<'_>) -> Result<(), E>,
{
    apply_to_text_spans_raw(artifact, f)
}

fn apply_to_text_spans_raw<E, F>(artifact: &mut Artifact, mut f: F) -> Result<(), E>
where
    F: FnMut(TextSpan<'_>) -> Result<(), E>,
{
    let mut index = 0usize;
    match artifact {
        Artifact::Txt(b) => {
            let span = TextSpan {
                id: SpanId(index as u32),
                index,
                text: &mut b.text,
            };
            f(span)
        }
        Artifact::Md(b) => {
            let span = TextSpan {
                id: SpanId(index as u32),
                index,
                text: &mut b.text,
            };
            f(span)
        }
        Artifact::Srt(b) => {
            for cue in &mut b.cues {
                let span = TextSpan {
                    id: SpanId(index as u32),
                    index,
                    text: &mut cue.text,
                };
                f(span)?;
                index += 1;
            }
            Ok(())
        }
        Artifact::Vtt(b) => {
            for block in &mut b.blocks {
                if let VttBlock::Cue { text, .. } = block {
                    let span = TextSpan {
                        id: SpanId(index as u32),
                        index,
                        text,
                    };
                    f(span)?;
                    index += 1;
                }
            }
            Ok(())
        }
        Artifact::Json(b) => apply_json_walk(&mut b.value, &mut index, &mut f),
        Artifact::Jsonl(b) => {
            for line in &mut b.lines {
                apply_json_walk(line, &mut index, &mut f)?;
            }
            Ok(())
        }
    }
}

fn apply_json_walk<E, F>(
    value: &mut serde_json::Value,
    index: &mut usize,
    f: &mut F,
) -> Result<(), E>
where
    F: FnMut(TextSpan<'_>) -> Result<(), E>,
{
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                let Some(v) = map.get_mut(&k) else {
                    continue;
                };
                if is_text_key(&k) {
                    if let Value::String(s) = v {
                        let span = TextSpan {
                            id: SpanId(*index as u32),
                            index: *index,
                            text: s,
                        };
                        f(span)?;
                        *index += 1;
                    }
                } else {
                    apply_json_walk(v, index, f)?;
                }
            }
            Ok(())
        }
        Value::Array(arr) => {
            for v in arr {
                apply_json_walk(v, index, f)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
