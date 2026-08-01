//! Deterministic local stub backend (tests / CI / dry pipelines).

use crate::artifact::{AudioRef, BackendInfo, Segment, SpeakerId, SpeakerTimeline};
use crate::backend::{Backend, DiarizeError, DiarizeRequest};

pub struct StubBackend;

impl Backend for StubBackend {
    fn provider(&self) -> &'static str {
        "stub"
    }

    fn infer(&self, req: &DiarizeRequest) -> Result<SpeakerTimeline, DiarizeError> {
        if !req.input.is_file() {
            return Err(DiarizeError::NotFound(format!(
                "input missing: {}",
                req.input.display()
            )));
        }
        // Synthetic duration from file size (no audio decode required).
        let meta = std::fs::metadata(&req.input).map_err(|e| DiarizeError::Other(e.to_string()))?;
        let duration = ((meta.len() as f64) / 16_000.0).clamp(2.0, 120.0);

        let speakers = vec![
            SpeakerId {
                id: "S0".into(),
            },
            SpeakerId {
                id: "S1".into(),
            },
        ];
        let mid = duration / 2.0;
        let segments = vec![
            Segment {
                speaker: "S0".into(),
                start: 0.0,
                end: mid,
                confidence: Some(1.0),
            },
            Segment {
                speaker: "S1".into(),
                start: mid,
                end: duration,
                confidence: Some(1.0),
            },
        ];

        Ok(SpeakerTimeline {
            version: 1,
            audio: AudioRef {
                path: req.input.clone(),
            },
            speakers,
            segments,
            overlaps: Vec::new(),
            embeddings: None,
            speech_regions: Vec::new(),
            backend: BackendInfo {
                provider: "stub".into(),
                model: req.backend.default_model().into(),
                version: Some("1".into()),
                device: req.device.clone().or_else(|| Some("cpu".into())),
            },
        })
    }
}
