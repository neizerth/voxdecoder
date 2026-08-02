//! Prepared media result.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedMedia {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreprocessResult {
    pub output: PreparedMedia,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extras: Vec<PreparedMedia>,
    /// Sidecar TimeMap when timing filters rewrote the clock (also listed in `extras`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timemap: Option<PathBuf>,
}
