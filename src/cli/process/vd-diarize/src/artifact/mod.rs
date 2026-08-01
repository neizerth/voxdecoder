//! SpeakerTimeline — canonical diarization artifact.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeakerTimeline {
    pub version: u32,
    pub audio: AudioRef,
    pub speakers: Vec<SpeakerId>,
    pub segments: Vec<Segment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlaps: Vec<Overlap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<Embeddings>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speech_regions: Vec<Region>,
    pub backend: BackendInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioRef {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeakerId {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub speaker: String,
    pub start: f64,
    pub end: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Overlap {
    pub start: f64,
    pub end: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speakers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Region {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Embeddings {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub per_speaker: Vec<SpeakerEmbedding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeakerEmbedding {
    pub speaker: String,
    pub model: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendInfo {
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

impl SpeakerTimeline {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("unsupported SpeakerTimeline version: {}", self.version));
        }
        if self.backend.provider.is_empty() || self.backend.model.is_empty() {
            return Err("backend.provider and backend.model are required".into());
        }
        for seg in &self.segments {
            if seg.end < seg.start {
                return Err(format!(
                    "segment end < start for speaker {}",
                    seg.speaker
                ));
            }
        }
        Ok(())
    }

    pub fn write_json(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    pub fn read_json(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let t: Self = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        t.validate()?;
        Ok(t)
    }
}

pub fn default_output_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.diarization.json"))
}
