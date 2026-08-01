//! pyannote provider — assets installable; local runtime TBD.

use crate::assets;
use crate::backend::{Backend, DiarizeError, DiarizeRequest};
use crate::artifact::SpeakerTimeline;

pub struct PyannoteBackend;

impl Backend for PyannoteBackend {
    fn provider(&self) -> &'static str {
        "pyannote"
    }

    fn infer(&self, req: &DiarizeRequest) -> Result<SpeakerTimeline, DiarizeError> {
        if !req.input.is_file() {
            return Err(DiarizeError::NotFound(format!(
                "input missing: {}",
                req.input.display()
            )));
        }
        if !assets::is_installed("pyannote") {
            return Err(DiarizeError::NotFound(
                "pyannote assets not installed; run: vd-diarize install pyannote".into(),
            ));
        }
        Err(DiarizeError::Unavailable(format!(
            "pyannote local runtime is not wired in this build (model {}); use --backend stub for pipelines/tests",
            req.backend.default_model()
        )))
    }
}
