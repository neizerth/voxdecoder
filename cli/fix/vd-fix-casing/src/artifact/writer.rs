//! Serialize artifact to disk (same type).

use std::fs;
use std::path::Path;

use super::load::{Artifact, ArtifactError};

pub fn write(artifact: &Artifact, path: &Path) -> Result<(), ArtifactError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ArtifactError::Io(e.to_string()))?;
    }
    let body = serialize(artifact)?;
    fs::write(path, body).map_err(|e| ArtifactError::Io(e.to_string()))
}

fn serialize(artifact: &Artifact) -> Result<String, ArtifactError> {
    match artifact {
        Artifact::Txt(b) => Ok(b.serialize()),
        Artifact::Md(b) => Ok(b.serialize()),
        Artifact::Srt(b) => Ok(b.serialize()),
        Artifact::Vtt(b) => Ok(b.serialize()),
        Artifact::Json(b) => b.serialize().map_err(ArtifactError::Message),
        Artifact::Jsonl(b) => b.serialize().map_err(ArtifactError::Message),
    }
}
