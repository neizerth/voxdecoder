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
        artifact_type: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<&'a str>,
    },
    Loading {
        #[serde(skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
    },
    Downloading {
        percent: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes_done: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes_total: Option<u64>,
    },
    Processing {
        #[serde(skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        span: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        span_total: Option<u32>,
    },
    Writing {
        #[serde(skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
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
                    ProgressEvent::Loading { percent } => {
                        let label = percent
                            .map_or_else(|| "loading".to_string(), |p| format!("loading {p}%"));
                        self.emit_phase(&mut err, tty, &label);
                    }
                    ProgressEvent::Downloading {
                        percent,
                        bytes_done,
                        bytes_total,
                    } => {
                        let label = match (bytes_done, bytes_total) {
                            (Some(d), Some(t)) => {
                                format!("downloading {percent}% ({d}/{t})")
                            }
                            _ => format!("downloading {percent}%"),
                        };
                        self.emit_phase(&mut err, tty, &label);
                    }
                    ProgressEvent::Processing {
                        percent,
                        span,
                        span_total,
                    } => {
                        let label = match (percent, span, span_total) {
                            (Some(p), Some(s), Some(t)) => {
                                format!("processing {p}% (span {s}/{t})")
                            }
                            (Some(p), _, _) => format!("processing {p}%"),
                            _ => "processing".to_string(),
                        };
                        self.emit_phase(&mut err, tty, &label);
                    }
                    ProgressEvent::Writing { percent } => {
                        let label = percent
                            .map_or_else(|| "writing".to_string(), |p| format!("writing {p}%"));
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
