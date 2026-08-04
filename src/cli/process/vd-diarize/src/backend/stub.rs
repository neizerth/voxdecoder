//! Deterministic local stub backend (tests / CI / dry pipelines).

use crate::artifact::{AudioRef, BackendInfo, Overlap, Segment, SpeakerId, SpeakerTimeline};
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
        let overlap_pad = (duration * 0.05).clamp(0.2, 1.0);
        let segments = vec![
            Segment {
                speaker: "S0".into(),
                start: 0.0,
                end: mid + overlap_pad,
                confidence: Some(1.0),
            },
            Segment {
                speaker: "S1".into(),
                start: mid - overlap_pad,
                end: duration,
                confidence: Some(1.0),
            },
        ];
        // ADR 0016: stub exposes a synthetic interruption window at the handoff.
        let overlaps = vec![Overlap {
            start: mid - overlap_pad,
            end: mid + overlap_pad,
            speakers: vec!["S0".into(), "S1".into()],
        }];

        Ok(SpeakerTimeline {
            version: 1,
            audio: AudioRef {
                path: req.input.clone(),
            },
            speakers,
            segments,
            overlaps,
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
