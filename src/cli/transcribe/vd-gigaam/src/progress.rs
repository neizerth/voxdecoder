//! Progress reporting on stderr (`--progress`).

use serde::Serialize;
use std::cell::Cell;
use std::io::{self, IsTerminal, Write};

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
    /// Text mode: last emit was an in-place `\r` phase line.
    phase_open: Cell<bool>,
}

impl Progress {
    pub fn new(mode: ProgressMode) -> Self {
        Self {
            mode,
            phase_open: Cell::new(false),
        }
    }

    fn finish_phase_line(&self, err: &mut impl Write) {
        if self.phase_open.get() {
            let _ = writeln!(err);
            self.phase_open.set(false);
        }
    }

    pub fn emit(&self, event: &ProgressEvent<'_>) {
        match self.mode {
            ProgressMode::None => {}
            ProgressMode::Json => {
                if let Ok(line) = serde_json::to_string(event) {
                    let _ = writeln!(io::stderr(), "{line}");
                }
            }
            ProgressMode::Text => {
                let mut err = io::stderr();
                match event {
                    ProgressEvent::Start { model, .. } => {
                        self.finish_phase_line(&mut err);
                        if let Some(m) = model {
                            let _ = writeln!(err, "start model={m}");
                        }
                    }
                    ProgressEvent::Phase { phase, percent, .. } => {
                        if err.is_terminal() {
                            let _ = write!(err, "\r\x1b[2K{phase} {percent}%");
                            let _ = err.flush();
                            self.phase_open.set(true);
                        } else {
                            let _ = writeln!(err, "{phase} {percent}%");
                        }
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
}
