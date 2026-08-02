//! TOML config file I/O.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{parse_layout_language, FileConfig};
use crate::types::{ParagraphDensity, ProgressFormat};

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawConfig {
    language: Option<String>,
    paragraph_density: Option<String>,
    use_timemap: Option<bool>,
    in_place: Option<bool>,
    progress: Option<String>,
    download_root: Option<String>,
}

pub fn load(path: &Path) -> Result<FileConfig, String> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: RawConfig = toml::from_str(&text).map_err(|e| e.to_string())?;
    Ok(FileConfig {
        language: raw
            .language
            .as_deref()
            .map(parse_layout_language)
            .transpose()?,
        paragraph_density: raw
            .paragraph_density
            .as_deref()
            .map(|p| {
                ParagraphDensity::parse(p)
                    .ok_or_else(|| format!("invalid paragraph_density in config: {p}"))
            })
            .transpose()?,
        use_timemap: raw.use_timemap,
        in_place: raw.in_place,
        progress: raw
            .progress
            .as_deref()
            .map(|p| {
                ProgressFormat::parse(p).ok_or_else(|| format!("invalid progress in config: {p}"))
            })
            .transpose()?,
        download_root: raw.download_root,
    })
}

pub fn save(path: &Path, cfg: &FileConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = RawConfig {
        language: cfg.language.map(|l| l.as_str().to_string()),
        paragraph_density: cfg.paragraph_density.map(|d| d.as_str().to_string()),
        use_timemap: cfg.use_timemap,
        in_place: cfg.in_place,
        progress: cfg.progress.map(|p| p.as_str().to_string()),
        download_root: cfg.download_root.clone(),
    };
    let text = toml::to_string_pretty(&raw).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}
