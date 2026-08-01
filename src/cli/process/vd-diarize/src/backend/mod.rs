//! Diarization backends (local inference).

mod nemo;
mod pyannote;
mod stub;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::artifact::SpeakerTimeline;

pub use stub::StubBackend;

#[derive(Debug, Clone)]
pub struct BackendSpec {
    pub provider: String,
    pub model: Option<String>,
}

impl BackendSpec {
    pub fn new(provider: impl Into<String>, model: Option<String>) -> Self {
        Self {
            provider: provider.into(),
            model,
        }
    }

    pub fn default_model(&self) -> &str {
        match self.provider.as_str() {
            "stub" => self.model.as_deref().unwrap_or("deterministic-v1"),
            "pyannote" => self
                .model
                .as_deref()
                .unwrap_or("speaker-diarization-3.1"),
            "nemo" => self.model.as_deref().unwrap_or("sortformer"),
            _ => self.model.as_deref().unwrap_or("default"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiarizeRequest {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub backend: BackendSpec,
    pub device: Option<String>,
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DiarizeError {
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Other(String),
}

impl DiarizeError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::NotFound(_) => 3,
            Self::Unavailable(_) | Self::Other(_) => 1,
        }
    }
}

pub trait Backend {
    fn provider(&self) -> &'static str;
    fn infer(&self, req: &DiarizeRequest) -> Result<SpeakerTimeline, DiarizeError>;
}

pub fn resolve_backend(spec: &BackendSpec) -> Result<Box<dyn Backend>, DiarizeError> {
    match spec.provider.as_str() {
        "stub" => Ok(Box::new(StubBackend)),
        "pyannote" => Ok(Box::new(pyannote::PyannoteBackend)),
        "nemo" => Ok(Box::new(nemo::NemoBackend)),
        other => Err(DiarizeError::Usage(format!("unknown backend provider: {other}"))),
    }
}

pub fn known_providers() -> &'static [&'static str] {
    &["stub", "pyannote", "nemo"]
}
