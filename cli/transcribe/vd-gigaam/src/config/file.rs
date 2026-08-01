//! TOML config file I/O.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::resolve::{Device, OutputFormat};
use super::FileConfig;

#[derive(Debug, Default, Serialize, Deserialize)]
struct RawConfig {
    model: Option<String>,
    device: Option<String>,
    fp16_encoder: Option<bool>,
    flash: Option<bool>,
    download_root: Option<String>,
    word_timestamps: Option<bool>,
    format: Option<String>,
}

pub fn load(path: &Path) -> Result<FileConfig, String> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: RawConfig = toml::from_str(&text).map_err(|e| e.to_string())?;
    Ok(FileConfig {
        model: raw.model,
        device: raw
            .device
            .as_deref()
            .map(|d| Device::parse(d).ok_or_else(|| format!("invalid device in config: {d}")))
            .transpose()?,
        fp16_encoder: raw.fp16_encoder,
        flash: raw.flash,
        download_root: raw.download_root,
        word_timestamps: raw.word_timestamps,
        format: raw
            .format
            .as_deref()
            .map(|f| OutputFormat::parse(f).ok_or_else(|| format!("invalid format in config: {f}")))
            .transpose()?,
    })
}

pub fn save(path: &Path, cfg: &FileConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = RawConfig {
        model: cfg.model.clone(),
        device: cfg.device.map(|d| d.as_str().to_string()),
        fp16_encoder: cfg.fp16_encoder,
        flash: cfg.flash,
        download_root: cfg.download_root.clone(),
        word_timestamps: cfg.word_timestamps,
        format: cfg.format.map(|f| f.as_str().to_string()),
    };
    let text = toml::to_string_pretty(&raw).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}
