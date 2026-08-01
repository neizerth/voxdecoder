//! Progress helpers.

use vd_progress::{Progress, ProgressEvent, ProgressMode};

pub fn start(mode: ProgressMode) -> Progress {
    Progress::new(mode)
}

pub fn emit_phase(progress: &Progress, phase: &str, percent: u8) {
    progress.emit(&ProgressEvent::phase(phase, percent));
}
