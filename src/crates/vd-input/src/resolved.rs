//! Resolved Runtime artifacts after input resolution.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    File,
    Url,
    Artifact,
    Blob,
}

/// Artifacts ready for Job planning (paths on the Runtime host).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedInput {
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl ResolvedInput {
    pub fn require_audio(&self) -> Result<&PathBuf, crate::InputError> {
        self.audio.as_ref().ok_or_else(|| {
            crate::InputError::Invalid("resolved input has no audio artifact".into())
        })
    }
}
