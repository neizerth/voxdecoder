//! Import request types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubtitlePolicy {
    #[default]
    Ignore,
    Prefer,
    Require,
}

impl SubtitlePolicy {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ignore" => Ok(Self::Ignore),
            "prefer" => Ok(Self::Prefer),
            "require" => Ok(Self::Require),
            other => Err(format!(
                "invalid subtitles policy '{other}' (ignore|prefer|require)"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Prefer => "prefer",
            Self::Require => "require",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UrlImportRequest {
    pub url: String,
    /// Resolver hint: `auto` / `youtube` / `direct` / `stub` / …
    pub provider: Option<String>,
    pub subtitles: SubtitlePolicy,
    pub metadata_only: bool,
    pub output_dir: PathBuf,
    pub overwrite: bool,
}
