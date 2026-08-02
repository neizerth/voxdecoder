//! Remap timeline-bearing JSON / SRT bodies using a [`TimeMap`].

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::timemap::TimeMap;
use super::ArtifactError;

/// Remap `segments[].{start,end}` and optional `words[].{start,end}` in a
/// GigaAM-style `{stem}.segments.json` (in place).
pub fn remap_segments_json(path: &Path, map: &TimeMap) -> Result<(), ArtifactError> {
    let raw = fs::read_to_string(path).map_err(|e| ArtifactError::Io(e.to_string()))?;
    let mut v: Value =
        serde_json::from_str(&raw).map_err(|e| ArtifactError::Parse(e.to_string()))?;
    remap_segments_value(&mut v, map);
    let out = serde_json::to_string_pretty(&v).map_err(|e| ArtifactError::Io(e.to_string()))?;
    fs::write(path, out).map_err(|e| ArtifactError::Io(e.to_string()))?;
    Ok(())
}

pub fn remap_segments_value(v: &mut Value, map: &TimeMap) {
    if let Some(segs) = v.get_mut("segments").and_then(|s| s.as_array_mut()) {
        for seg in segs {
            remap_start_end(seg, map);
        }
    }
    if let Some(words) = v.get_mut("words").and_then(|w| w.as_array_mut()) {
        for w in words {
            remap_start_end(w, map);
        }
    }
}

fn remap_start_end(obj: &mut Value, map: &TimeMap) {
    let start = obj.get("start").and_then(|x| x.as_f64());
    let end = obj.get("end").and_then(|x| x.as_f64());
    if let (Some(s), Some(e)) = (start, end) {
        let (ns, ne) = map.remap_interval(s, e);
        if let Some(o) = obj.as_object_mut() {
            o.insert("start".into(), Value::from(ns));
            o.insert("end".into(), Value::from(ne));
        }
    }
}

/// Remap SRT cue timings in place (`HH:MM:SS,mmm --> HH:MM:SS,mmm`).
pub fn remap_srt_file(path: &Path, map: &TimeMap) -> Result<(), ArtifactError> {
    let raw = fs::read_to_string(path).map_err(|e| ArtifactError::Io(e.to_string()))?;
    let out = remap_srt_text(&raw, map)?;
    fs::write(path, out).map_err(|e| ArtifactError::Io(e.to_string()))?;
    Ok(())
}

pub fn remap_srt_text(raw: &str, map: &TimeMap) -> Result<String, ArtifactError> {
    let mut out = String::with_capacity(raw.len());
    for line in raw.lines() {
        if let Some((left, right)) = line.split_once(" --> ") {
            if let (Some(a), Some(b)) = (parse_srt_ts(left.trim()), parse_srt_ts(right.trim())) {
                let (na, nb) = map.remap_interval(a, b);
                out.push_str(&format_srt_ts(na));
                out.push_str(" --> ");
                out.push_str(&format_srt_ts(nb));
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
}

fn parse_srt_ts(s: &str) -> Option<f64> {
    // HH:MM:SS,mmm or HH:MM:SS.mmm
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let sec: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

fn format_srt_ts(t: f64) -> String {
    let t = t.max(0.0);
    let ms_total = (t * 1000.0).round() as u64;
    let ms = ms_total % 1000;
    let total_s = ms_total / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timemap::TimeMap;

    #[test]
    fn remap_segments_scales() {
        let map = TimeMap::uniform(10.0, 20.0);
        let mut v = serde_json::json!({
            "text": "hi",
            "segments": [{ "text": "hi", "start": 0.0, "end": 10.0 }],
            "words": [{ "text": "hi", "start": 2.0, "end": 4.0 }]
        });
        remap_segments_value(&mut v, &map);
        assert!((v["segments"][0]["end"].as_f64().unwrap() - 20.0).abs() < 1e-9);
        assert!((v["words"][0]["start"].as_f64().unwrap() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn remap_srt_line() {
        let map = TimeMap::uniform(10.0, 20.0);
        let raw = "1\n00:00:01,000 --> 00:00:02,000\nhi\n";
        let out = remap_srt_text(raw, &map).unwrap();
        assert!(out.contains("00:00:02,000 --> 00:00:04,000"));
    }
}
