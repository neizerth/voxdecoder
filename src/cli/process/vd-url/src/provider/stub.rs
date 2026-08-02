//! Stub provider for tests (no network).

use std::fs;

use serde_json::json;

use super::MediaProvider;
use crate::artifact;
use crate::import::{
    prepare_output_dir, ArtifactHandle, ImportError, ImportResult, ProviderId, SubtitlePolicy,
    UrlImportRequest,
};

pub struct StubProvider;

impl MediaProvider for StubProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Stub
    }

    fn supports_subtitles(&self) -> bool {
        true
    }

    fn resolve(&self, request: &UrlImportRequest) -> Result<ImportResult, ImportError> {
        prepare_output_dir(&request.output_dir, request.overwrite)?;

        let meta = json!({
            "import": { "provider": "stub" },
            "url": request.url,
            "title": "stub-title",
            "duration": 12.0,
            "language": "en",
            "subtitles_available": true,
            "chapters": [],
        });
        let metadata_path = artifact::write_metadata(&request.output_dir, &meta)?;

        let audio = if request.metadata_only {
            None
        } else {
            let path = request.output_dir.join("audio.wav");
            // Minimal WAV header + silence placeholder (44-byte header, 0 data ok for tests).
            let wav = minimal_wav();
            if path.exists() && !request.overwrite {
                return Err(ImportError::Exists(path));
            }
            fs::write(&path, wav).map_err(|e| ImportError::Io(e.to_string()))?;
            Some(ArtifactHandle::new("audio", "audio", path))
        };

        let subtitle = match request.subtitles {
            SubtitlePolicy::Ignore => None,
            SubtitlePolicy::Prefer | SubtitlePolicy::Require => {
                let path = request.output_dir.join("subtitles.vtt");
                let body = "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nstub\n";
                fs::write(&path, body).map_err(|e| ImportError::Io(e.to_string()))?;
                Some(ArtifactHandle::new("subtitle", "subtitle", path))
            }
        };

        Ok(ImportResult {
            provider: ProviderId::Stub,
            audio,
            metadata: ArtifactHandle::new("metadata", "metadata", metadata_path),
            subtitle,
        })
    }
}

fn minimal_wav() -> Vec<u8> {
    // 8-byte silent PCM WAV (mono 8kHz) — enough for file presence checks.
    let mut v = Vec::with_capacity(44);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&36u32.to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // mono
    v.extend_from_slice(&8000u32.to_le_bytes());
    v.extend_from_slice(&8000u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&8u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}
