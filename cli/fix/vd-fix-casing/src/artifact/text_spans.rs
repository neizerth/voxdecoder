//! Apply a callback to each mutable transcript `TextSpan`. Structure is unreachable.

use super::formats::{is_text_key, VttBlock};
use super::load::Artifact;
use crate::types::TextSpan;

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

/// Visit each mutable transcript text span. Structure fields are not exposed.
pub fn apply_to_text_spans<E, F>(artifact: &mut Artifact, mut f: F) -> Result<(), E>
where
    F: FnMut(TextSpan<'_>) -> Result<(), E>,
{
    match artifact {
        Artifact::Txt(b) => f(TextSpan { text: &mut b.text }),
        Artifact::Md(b) => f(TextSpan { text: &mut b.text }),
        Artifact::Srt(b) => {
            for cue in &mut b.cues {
                f(TextSpan {
                    text: &mut cue.text,
                })?;
            }
            Ok(())
        }
        Artifact::Vtt(b) => {
            for block in &mut b.blocks {
                if let VttBlock::Cue { text, .. } = block {
                    f(TextSpan { text })?;
                }
            }
            Ok(())
        }
        Artifact::Json(b) => apply_json_walk(&mut b.value, &mut f),
        Artifact::Jsonl(b) => {
            for line in &mut b.lines {
                apply_json_walk(line, &mut f)?;
            }
            Ok(())
        }
    }
}

fn apply_json_walk<E, F>(value: &mut serde_json::Value, f: &mut F) -> Result<(), E>
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
                        f(TextSpan { text: s })?;
                    }
                } else {
                    apply_json_walk(v, f)?;
                }
            }
            Ok(())
        }
        Value::Array(arr) => {
            for v in arr {
                apply_json_walk(v, f)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
