//! Progress helpers (`start` → `phase`* → `done`).

use std::path::Path;

use vd_progress::{Progress, ProgressEvent, ProgressMode};

pub fn start(mode: ProgressMode) -> Progress {
    Progress::new(mode)
}

pub fn emit_start(progress: &Progress, _recipe_count: usize, runner: Option<&str>) {
    progress.emit(&ProgressEvent::Start {
        input: None,
        output: None,
        artifact_type: Some("derived"),
        language: None,
        model: Some(runner.unwrap_or("recipe-graph")),
        device: None,
        path: None,
    });
}

pub fn emit_phase(progress: &Progress, phase: &str, percent: u8) {
    progress.emit(&ProgressEvent::phase(phase, percent));
}

/// Per-node progress while executing the recipe graph.
pub fn emit_node(
    progress: &Progress,
    phase: &str,
    percent: u8,
    node_index: u32,
    node_total: u32,
) {
    progress.emit(&ProgressEvent::phase_span(
        phase,
        percent,
        node_index,
        node_total,
    ));
}

pub fn emit_done(progress: &Progress, primary_output: Option<&Path>, duration_sec: f64) {
    let out = primary_output.map(|p| p.display().to_string());
    progress.emit(&ProgressEvent::Done {
        output: out.as_deref(),
        model: None,
        path: out.as_deref(),
        duration_sec: Some(duration_sec),
        char_count: None,
    });
}
