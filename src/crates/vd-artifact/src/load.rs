//! Load artifact from disk.

use std::fs;
use std::path::Path;

use super::detect::detect_type;
use super::formats::{JsonBody, JsonlBody, MdBody, SrtBody, TxtBody, VttBody};
use crate::types::ArtifactType;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("{0}")]
    Message(String),
    #[error("unsupported artifact type")]
    UnsupportedType,
    #[error("input missing or unreadable: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
}

impl ArtifactError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::UnsupportedType | Self::Io(_) => 3,
            Self::Message(_) | Self::Parse(_) => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Artifact {
    Txt(TxtBody),
    Json(JsonBody),
    Jsonl(JsonlBody),
    Srt(SrtBody),
    Vtt(VttBody),
    Md(MdBody),
}

impl Artifact {
    pub fn artifact_type(&self) -> ArtifactType {
        match self {
            Self::Txt(_) => ArtifactType::Txt,
            Self::Json(_) => ArtifactType::Json,
            Self::Jsonl(_) => ArtifactType::Jsonl,
            Self::Srt(_) => ArtifactType::Srt,
            Self::Vtt(_) => ArtifactType::Vtt,
            Self::Md(_) => ArtifactType::Md,
        }
    }
}

pub fn load(path: &Path) -> Result<Artifact, ArtifactError> {
    let kind = detect_type(path).ok_or(ArtifactError::UnsupportedType)?;
    let raw = fs::read_to_string(path).map_err(|e| ArtifactError::Io(e.to_string()))?;
    load_from_str(kind, &raw)
}

pub fn load_from_str(kind: ArtifactType, raw: &str) -> Result<Artifact, ArtifactError> {
    match kind {
        ArtifactType::Txt => Ok(Artifact::Txt(TxtBody::parse(raw))),
        ArtifactType::Md => Ok(Artifact::Md(MdBody::parse(raw))),
        ArtifactType::Json => Ok(Artifact::Json(
            JsonBody::parse(raw).map_err(ArtifactError::Message)?,
        )),
        ArtifactType::Jsonl => Ok(Artifact::Jsonl(
            JsonlBody::parse(raw).map_err(ArtifactError::Message)?,
        )),
        ArtifactType::Srt => Ok(Artifact::Srt(SrtBody::parse(raw))),
        ArtifactType::Vtt => Ok(Artifact::Vtt(VttBody::parse(raw))),
    }
}
