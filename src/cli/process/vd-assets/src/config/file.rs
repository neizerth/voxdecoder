//! TOML config I/O.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::FileConfig;
use crate::types::ProgressFormat;

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawConfig {
    progress: Option<String>,
    ocr: Option<bool>,
}

pub fn load(path: &Path) -> Result<FileConfig, String> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: RawConfig = toml::from_str(&text).map_err(|e| e.to_string())?;
    Ok(FileConfig {
        progress: raw
            .progress
            .as_deref()
            .map(|p| {
                ProgressFormat::parse(p).ok_or_else(|| format!("invalid progress in config: {p}"))
            })
            .transpose()?,
        ocr: raw.ocr,
    })
}

pub fn save(path: &Path, cfg: &FileConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = RawConfig {
        progress: cfg.progress.map(|p| p.as_str().to_string()),
        ocr: cfg.ocr,
    };
    let text = toml::to_string_pretty(&raw).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}
