//! Filter chain load / expand / validate.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::filter::{known_operation, FilterSpec, RawFilter};
use super::PreprocessError;

#[derive(Debug, Clone, Deserialize)]
pub struct ChainFile {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub filters: Vec<RawFilter>,
}

pub fn load_chain_file(path: &Path) -> Result<ChainFile, PreprocessError> {
    let text = fs::read_to_string(path).map_err(|e| {
        if path.exists() {
            PreprocessError::Other(format!("{}: {e}", path.display()))
        } else {
            PreprocessError::NotFound(format!("chain missing: {}", path.display()))
        }
    })?;
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        serde_json::from_str(&text)
            .map_err(|e| PreprocessError::Usage(format!("chain json: {e}")))
    } else {
        serde_yaml::from_str(&text)
            .map_err(|e| PreprocessError::Usage(format!("chain yaml: {e}")))
    }
}

pub fn expand_and_validate(
    raw: Vec<RawFilter>,
    default_provider: &str,
) -> Result<Vec<FilterSpec>, PreprocessError> {
    if raw.is_empty() {
        return Err(PreprocessError::Usage("no filters specified".into()));
    }
    let mut out = Vec::with_capacity(raw.len());
    for item in raw {
        let spec = item.expand(default_provider)?;
        if !known_operation(&spec.operation) {
            return Err(PreprocessError::Usage(format!(
                "unknown filter operation: {}",
                spec.operation
            )));
        }
        match spec.provider.as_str() {
            "stub" | "ffmpeg" | "sox" | "deepfilternet" | "rnnoise" | "demucs" => {}
            other => {
                return Err(PreprocessError::Usage(format!(
                    "unknown media provider: {other}"
                )));
            }
        }
        out.push(spec);
    }
    Ok(out)
}
