//! Serialize transcript into txt | json | srt | vtt.

use serde::Serialize;

use crate::config::resolve::OutputFormat;
use crate::gigaam::model::Transcript;

#[derive(Debug, Serialize)]
pub struct JsonResult<'a> {
    pub text: &'a str,
    pub segments: &'a [crate::gigaam::model::Segment],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<&'a [crate::gigaam::model::Word]>,
}

pub fn render(format: OutputFormat, transcript: &Transcript) -> String {
    match format {
        OutputFormat::Txt => transcript.text.clone(),
        OutputFormat::Json => {
            let body = JsonResult {
                text: &transcript.text,
                segments: &transcript.segments,
                words: transcript.words.as_deref(),
            };
            serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".into())
        }
        OutputFormat::Srt => render_srt(transcript),
        OutputFormat::Vtt => render_vtt(transcript),
    }
}

fn render_srt(t: &Transcript) -> String {
    if t.segments.is_empty() {
        return format!("1\n{} --> {}\n{}\n", ts_srt(0.0), ts_srt(0.0), t.text);
    }
    let mut out = String::new();
    for (i, seg) in t.segments.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            ts_srt(seg.start),
            ts_srt(seg.end),
            seg.text
        ));
    }
    out
}

fn render_vtt(t: &Transcript) -> String {
    let mut out = String::from("WEBVTT\n\n");
    if t.segments.is_empty() {
        out.push_str(&format!(
            "{} --> {}\n{}\n",
            ts_vtt(0.0),
            ts_vtt(0.0),
            t.text
        ));
        return out;
    }
    for seg in &t.segments {
        out.push_str(&format!(
            "{} --> {}\n{}\n\n",
            ts_vtt(seg.start),
            ts_vtt(seg.end),
            seg.text
        ));
    }
    out
}

fn ts_srt(sec: f64) -> String {
    let (h, m, s, ms) = split_ts(sec);
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

fn ts_vtt(sec: f64) -> String {
    let (h, m, s, ms) = split_ts(sec);
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

fn split_ts(sec: f64) -> (u64, u64, u64, u64) {
    let ms_total = (sec * 1000.0).round().max(0.0) as u64;
    let ms = ms_total % 1000;
    let total_s = ms_total / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    (h, m, s, ms)
}
