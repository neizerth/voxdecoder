//! Progress helpers for Job execution.

use std::path::Path;

use vd_progress::{Progress, ProgressEvent};

use crate::job::{Capability, ResolvedStep};

pub fn emit_start(progress: &Progress, input: Option<&Path>, model: Option<&str>) {
    let input_s = input.map(|p| p.display().to_string());
    progress.emit(&ProgressEvent::Start {
        input: input_s.as_deref(),
        output: None,
        artifact_type: Some("job"),
        language: None,
        model,
        device: None,
        path: None,
    });
}

pub fn emit_step_start(progress: &Progress, step: &ResolvedStep, total: u32, overall: u8) {
    let path = step.input.as_ref().map(|p| p.display().to_string());
    // Phase has no `step`/`id`/`name` fields yet in vd-progress — encode in phase + path.
    let phase = format!("step_start:{}", step.capability.as_str());
    progress.emit(&ProgressEvent::Phase {
        phase: &phase,
        percent: Some(overall),
        span: Some(step.index),
        span_total: Some(total),
        segment: None,
        segment_total: None,
        bytes_done: None,
        bytes_total: None,
    });
    let _ = path;
    let _ = step.id.as_ref();
    let _ = step.name.as_ref();
}

pub fn emit_step_done(
    progress: &Progress,
    step: &ResolvedStep,
    total: u32,
    overall: u8,
    output: &Path,
) {
    let phase = format!("step_done:{}", step.capability.as_str());
    let out = output.display().to_string();
    progress.emit(&ProgressEvent::Phase {
        phase: &phase,
        percent: Some(overall),
        span: Some(step.index),
        span_total: Some(total),
        segment: None,
        segment_total: None,
        bytes_done: None,
        bytes_total: None,
    });
    let _ = out;
}

pub fn emit_step_skipped(progress: &Progress, step: &ResolvedStep, total: u32, overall: u8) {
    let phase = format!("step_skipped:{}", step.capability.as_str());
    progress.emit(&ProgressEvent::Phase {
        phase: &phase,
        percent: Some(overall),
        span: Some(step.index),
        span_total: Some(total),
        segment: None,
        segment_total: None,
        bytes_done: None,
        bytes_total: None,
    });
}

pub fn emit_done(progress: &Progress, output: Option<&Path>, duration_sec: f64) {
    let out = output.map(|p| p.display().to_string());
    progress.emit(&ProgressEvent::Done {
        output: out.as_deref(),
        model: None,
        path: out.as_deref(),
        duration_sec: Some(duration_sec),
        char_count: None,
    });
}

pub fn emit_error(progress: &Progress, code: &str, message: &str) {
    progress.emit(&ProgressEvent::Error { code, message });
}

pub fn overall_percent(completed: u32, total: u32) -> u8 {
    if total == 0 {
        return 100;
    }
    ((completed * 100) / total).min(100) as u8
}

pub fn engine_from_steps(steps: &[ResolvedStep]) -> Option<String> {
    steps.iter().find_map(|s| {
        if s.capability == Capability::Transcribe {
            s.options
                .get("engine")
                .and_then(crate::job::ArgValue::as_string)
        } else {
            None
        }
    })
}
