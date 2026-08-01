//! Core diarize flow (shared by CLI and library callers).

use std::path::PathBuf;
use std::time::Instant;

use vd_progress::{Progress, ProgressEvent, ProgressMode};

use crate::artifact::{self, SpeakerTimeline};
use crate::backend::{self, DiarizeError, DiarizeRequest};
use crate::status;

pub struct DiarizeOutcome {
    pub timeline: SpeakerTimeline,
    pub output: PathBuf,
}

pub fn diarize(
    req: &DiarizeRequest,
    progress: ProgressMode,
    overwrite: bool,
) -> Result<DiarizeOutcome, DiarizeError> {
    let progress = Progress::new(progress);
    status::emit_start(&progress, &req.input, &req.backend.provider);

    let started = Instant::now();
    let backend = backend::resolve_backend(&req.backend)?;
    progress.emit(&ProgressEvent::phase("resolving_backend", 10));
    progress.emit(&ProgressEvent::phase("resolving_assets", 20));
    progress.emit(&ProgressEvent::phase("loading_backend", 40));
    progress.emit(&ProgressEvent::phase("inferring", 60));

    let timeline = backend.infer(req)?;
    timeline
        .validate()
        .map_err(DiarizeError::Other)?;

    let output = req
        .output
        .clone()
        .unwrap_or_else(|| artifact::default_output_path(&req.input));

    if output.exists() && !overwrite {
        return Err(DiarizeError::Usage(format!(
            "output exists (pass --overwrite): {}",
            output.display()
        )));
    }

    progress.emit(&ProgressEvent::phase("writing", 90));
    timeline
        .write_json(&output)
        .map_err(DiarizeError::Other)?;

    status::emit_done(
        &progress,
        &output,
        started.elapsed().as_secs_f64(),
    );
    Ok(DiarizeOutcome { timeline, output })
}
