//! Progress helpers.

use vd_progress::{Progress, ProgressEvent, ProgressMode};

/// Prefer `VD_PROGRESS_SNAPSHOT` when set (Runtime / pipeline) so `-q` still updates `get_job`.
pub fn start(mode: ProgressMode) -> Progress {
    Progress::from_env(mode)
}

pub fn emit_phase(progress: &Progress, phase: &str, percent: u8) {
    progress.emit(&ProgressEvent::phase(phase, percent));
}

pub fn emit_filter(
    progress: &Progress,
    operation: &str,
    local_pct: u8,
    filter_index: u32,
    filter_total: u32,
) {
    let phase = format!("preprocess:{operation}");
    // Map this filter's local 0–100 into the overall 10–95 window across the chain.
    let span = if filter_total == 0 {
        85
    } else {
        85 / filter_total
    };
    let base = 10 + filter_index.saturating_sub(1).saturating_mul(span);
    let overall = (base + (u32::from(local_pct) * span) / 100).min(95) as u8;
    progress.emit(&ProgressEvent::phase_span(
        &phase,
        overall,
        filter_index.max(1),
        filter_total.max(1),
    ));
}
