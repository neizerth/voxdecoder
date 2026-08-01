//! Progress helpers.

use std::path::Path;

use vd_progress::{Progress, ProgressEvent};

pub fn emit_start(progress: &Progress, input: &Path, provider: &str) {
    let input_s = input.display().to_string();
    progress.emit(&ProgressEvent::Start {
        input: Some(input_s.as_str()),
        output: None,
        artifact_type: Some("speaker_timeline"),
        language: None,
        model: Some(provider),
        device: None,
        path: None,
    });
}

pub fn emit_done(progress: &Progress, output: &Path, duration_sec: f64) {
    let out = output.display().to_string();
    progress.emit(&ProgressEvent::Done {
        output: Some(out.as_str()),
        model: None,
        path: Some(out.as_str()),
        duration_sec: Some(duration_sec),
        char_count: None,
    });
}
