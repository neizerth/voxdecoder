//! Import errors.

use std::path::PathBuf;

use thiserror::Error;

use super::ProviderId;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("unknown provider hint: {0}")]
    UnknownProvider(String),
    #[error("no provider matches URL: {0}")]
    NoProvider(String),
    #[error("provider '{0}' does not support required subtitles")]
    SubtitlesUnsupported(ProviderId),
    #[error("subtitles required but none available")]
    SubtitlesRequired,
    #[error("tool unavailable: {0}")]
    Unavailable(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("output exists (use --overwrite): {0}")]
    Exists(PathBuf),
    #[error("I/O: {0}")]
    Io(String),
    #[error("{0}")]
    Usage(String),
}

impl ImportError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidUrl(_)
            | Self::UnknownProvider(_)
            | Self::NoProvider(_)
            | Self::SubtitlesUnsupported(_)
            | Self::Usage(_) => 2,
            _ => 1,
        }
    }
}
