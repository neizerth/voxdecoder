//! NVIDIA NeMo provider — assets installable; local runtime TBD.

use crate::assets;
use crate::artifact::SpeakerTimeline;
use crate::backend::{Backend, DiarizeError, DiarizeRequest};

pub struct NemoBackend;

impl Backend for NemoBackend {
    fn provider(&self) -> &'static str {
        "nemo"
    }

    fn infer(&self, req: &DiarizeRequest) -> Result<SpeakerTimeline, DiarizeError> {
        if !req.input.is_file() {
            return Err(DiarizeError::NotFound(format!(
                "input missing: {}",
                req.input.display()
            )));
        }
        if !assets::is_installed("nemo") {
            return Err(DiarizeError::NotFound(
                "nemo assets not installed; run: vd-diarize install nemo".into(),
            ));
        }
        Err(DiarizeError::Unavailable(format!(
            "nemo local runtime is not wired in this build (model {}); use --backend stub for pipelines/tests",
            req.backend.default_model()
        )))
    }
}
