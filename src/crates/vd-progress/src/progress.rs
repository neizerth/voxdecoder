//! Progress reporting on stderr (`--progress`).
//!
//! Unified NDJSON scheme for `vd-fix-*` and transcription CLIs:
//! `start` → `phase`* → `done` | `error`.

use serde::Serialize;
use std::cell::Cell;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    Text,
    Json,
    None,
}

impl ProgressMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ProgressEvent<'a> {
    Start {
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        artifact_type: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        device: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<&'a str>,
    },
    /// Mid-lifecycle work unit (`loading`, `downloading`, `processing`, `transcribing`, …).
    Phase {
        phase: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        span: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        span_total: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        segment: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        segment_total: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes_done: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes_total: Option<u64>,
    },
    Done {
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_sec: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        char_count: Option<usize>,
    },
    Error {
        code: &'a str,
        message: &'a str,
    },
}

impl<'a> ProgressEvent<'a> {
    /// Phase with percent only (loading / writing / …).
    pub fn phase(phase: &'a str, percent: u8) -> Self {
        Self::Phase {
            phase,
            percent: Some(percent),
            span: None,
            span_total: None,
            segment: None,
            segment_total: None,
            bytes_done: None,
            bytes_total: None,
        }
    }

    /// Phase with span counters (fix CLIs).
    pub fn phase_span(phase: &'a str, percent: u8, span: u32, span_total: u32) -> Self {
        Self::Phase {
            phase,
            percent: Some(percent),
            span: Some(span),
            span_total: Some(span_total),
            segment: None,
            segment_total: None,
            bytes_done: None,
            bytes_total: None,
        }
    }

    /// Phase with segment/chunk counters (ASR windows).
    pub fn phase_segment(phase: &'a str, percent: u8, segment: u32, segment_total: u32) -> Self {
        Self::Phase {
            phase,
            percent: Some(percent),
            span: None,
            span_total: None,
            segment: Some(segment),
            segment_total: Some(segment_total),
            bytes_done: None,
            bytes_total: None,
        }
    }

    /// Phase with download bytes.
    pub fn phase_download(phase: &'a str, percent: u8, done: u64, total: Option<u64>) -> Self {
        Self::Phase {
            phase,
            percent: Some(percent),
            span: None,
            span_total: None,
            segment: None,
            segment_total: None,
            bytes_done: Some(done),
            bytes_total: total,
        }
    }
}

pub struct Progress {
    mode: ProgressMode,
    phase_open: Cell<bool>,
    /// Optional atomic snapshot file (`{"percent":N,"phase":"…"}`) for Runtime/agents.
    snapshot: Option<PathBuf>,
}

impl Progress {
    pub fn new(mode: ProgressMode) -> Self {
        Self {
            mode,
            phase_open: Cell::new(false),
            snapshot: None,
        }
    }

    /// Also write a small JSON snapshot on each event (for `get_job` / live UI).
    pub fn with_snapshot(mode: ProgressMode, path: impl Into<PathBuf>) -> Self {
        Self {
            mode,
            phase_open: Cell::new(false),
            snapshot: Some(path.into()),
        }
    }

    /// Prefer `VD_PROGRESS_SNAPSHOT` when set (Runtime child tools); otherwise `mode` only.
    pub fn from_env(mode: ProgressMode) -> Self {
        match std::env::var_os("VD_PROGRESS_SNAPSHOT") {
            Some(p) if !p.is_empty() => Self::with_snapshot(mode, PathBuf::from(p)),
            _ => Self::new(mode),
        }
    }

    fn finish_phase_line(&self, err: &mut impl Write) {
        if self.phase_open.get() {
            let _ = writeln!(err);
            self.phase_open.set(false);
        }
    }

    fn remap_percent(local: Option<u8>) -> Option<u8> {
        let Some(local) = local else {
            return None;
        };
        let base: u8 = std::env::var("VD_PROGRESS_STEP_BASE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let span: u8 = std::env::var("VD_PROGRESS_STEP_SPAN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);
        let mapped = u16::from(base) + (u16::from(local) * u16::from(span)) / 100;
        Some(mapped.min(100) as u8)
    }

    fn write_snapshot(&self, event: &ProgressEvent<'_>) {
        let Some(path) = &self.snapshot else {
            return;
        };
        let mut processed: Option<u64> = None;
        let mut total: Option<u64> = None;
        let mut unit: Option<&str> = None;
        let (percent, phase): (Option<u8>, Option<&str>) = match event {
            ProgressEvent::Start { .. } => (Some(0), Some("start")),
            ProgressEvent::Phase {
                phase,
                percent,
                span,
                span_total,
                segment,
                segment_total,
                bytes_done,
                bytes_total,
            } => {
                if let (Some(d), t) = (*bytes_done, *bytes_total) {
                    processed = Some(d);
                    total = t;
                    unit = Some("byte");
                } else if let (Some(s), Some(t)) = (*segment, *segment_total) {
                    processed = Some(u64::from(s));
                    total = Some(u64::from(t));
                    unit = Some("chunk");
                } else if let (Some(s), Some(t)) = (*span, *span_total) {
                    processed = Some(u64::from(s));
                    total = Some(u64::from(t));
                    unit = Some("step");
                }
                (*percent, Some(*phase))
            }
            ProgressEvent::Done { .. } => (Some(100), Some("done")),
            ProgressEvent::Error { .. } => (None, Some("error")),
        };
        let percent = Self::remap_percent(percent);
        let mut body = serde_json::json!({
            "percent": percent,
            "phase": phase,
        });
        if let Some(obj) = body.as_object_mut() {
            if let Some(p) = processed {
                obj.insert("processed".into(), serde_json::json!(p));
            }
            if let Some(t) = total {
                obj.insert("total".into(), serde_json::json!(t));
            }
            if let Some(u) = unit {
                obj.insert("unit".into(), serde_json::json!(u));
            }
        }
        let Ok(raw) = serde_json::to_vec_pretty(&body) else {
            return;
        };
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, raw).is_ok() {
            let _ = fs::rename(&tmp, path);
        }
    }

    pub fn emit(&self, event: &ProgressEvent<'_>) {
        self.write_snapshot(event);
        match self.mode {
            ProgressMode::None => {}
            ProgressMode::Json => {
                if let Ok(line) = serde_json::to_string(event) {
                    let _ = writeln!(io::stderr(), "{line}");
                }
            }
            ProgressMode::Text => {
                let mut err = io::stderr();
                let tty = err.is_terminal();
                match event {
                    ProgressEvent::Start { model, .. } => {
                        self.finish_phase_line(&mut err);
                        if let Some(m) = model {
                            let _ = writeln!(err, "start model={m}");
                        } else {
                            let _ = writeln!(err, "start");
                        }
                    }
                    ProgressEvent::Phase {
                        phase,
                        percent,
                        span,
                        span_total,
                        segment,
                        segment_total,
                        bytes_done,
                        bytes_total,
                    } => {
                        let label = phase_label(
                            phase,
                            *percent,
                            *span,
                            *span_total,
                            *segment,
                            *segment_total,
                            *bytes_done,
                            *bytes_total,
                        );
                        self.emit_phase(&mut err, tty, &label);
                    }
                    ProgressEvent::Done {
                        output,
                        model,
                        path,
                        ..
                    } => {
                        self.finish_phase_line(&mut err);
                        if let Some(o) = output {
                            let _ = writeln!(err, "done {o}");
                        } else if let Some(m) = model {
                            let _ = writeln!(err, "done {m}");
                        } else if let Some(p) = path {
                            let _ = writeln!(err, "done {p}");
                        } else {
                            let _ = writeln!(err, "done");
                        }
                    }
                    ProgressEvent::Error { code, message } => {
                        self.finish_phase_line(&mut err);
                        let _ = writeln!(err, "error {code}: {message}");
                    }
                }
            }
        }
    }

    fn emit_phase(&self, err: &mut impl Write, tty: bool, label: &str) {
        if tty {
            let _ = write!(err, "\r\x1b[2K{label}");
            let _ = err.flush();
            self.phase_open.set(true);
        } else {
            let _ = writeln!(err, "{label}");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn phase_label(
    phase: &str,
    percent: Option<u8>,
    span: Option<u32>,
    span_total: Option<u32>,
    segment: Option<u32>,
    segment_total: Option<u32>,
    bytes_done: Option<u64>,
    bytes_total: Option<u64>,
) -> String {
    let pct = percent.map(|p| format!(" {p}%")).unwrap_or_default();
    let detail = if let (Some(d), Some(t)) = (bytes_done, bytes_total) {
        format!(" ({d}/{t})")
    } else if let (Some(s), Some(t)) = (span, span_total) {
        format!(" (span {s}/{t})")
    } else if let (Some(s), Some(t)) = (segment, segment_total) {
        format!(" (segment {s}/{t})")
    } else {
        String::new()
    };
    format!("{phase}{pct}{detail}")
}
