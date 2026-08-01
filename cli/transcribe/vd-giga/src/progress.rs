//! Progress reporting on stderr (`--progress`).

use serde::Serialize;
use std::io::{self, Write};

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
            "none" => Some(Self::None),
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
        model: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        device: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<&'a str>,
    },
    Phase {
        phase: &'a str,
        percent: u8,
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

pub struct Progress {
    mode: ProgressMode,
}

impl Progress {
    pub fn new(mode: ProgressMode) -> Self {
        Self { mode }
    }

    pub fn emit(&self, event: &ProgressEvent<'_>) {
        match self.mode {
            ProgressMode::None => {}
            ProgressMode::Json => {
                if let Ok(line) = serde_json::to_string(event) {
                    let _ = writeln!(io::stderr(), "{line}");
                }
            }
            ProgressMode::Text => match event {
                ProgressEvent::Start { model, .. } => {
                    if let Some(m) = model {
                        let _ = writeln!(io::stderr(), "start model={m}");
                    }
                }
                ProgressEvent::Phase { phase, percent, .. } => {
                    let _ = writeln!(io::stderr(), "{phase} {percent}%");
                }
                ProgressEvent::Done {
                    output,
                    model,
                    path,
                    ..
                } => {
                    if let Some(o) = output {
                        let _ = writeln!(io::stderr(), "done {o}");
                    } else if let Some(m) = model {
                        let _ = writeln!(io::stderr(), "done {m}");
                    } else if let Some(p) = path {
                        let _ = writeln!(io::stderr(), "done {p}");
                    }
                }
                ProgressEvent::Error { code, message } => {
                    let _ = writeln!(io::stderr(), "error {code}: {message}");
                }
            },
        }
    }
}
