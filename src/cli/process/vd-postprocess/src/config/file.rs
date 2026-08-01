//! Persist config.toml.

use std::fs;
use std::path::Path;

use super::FileConfig;

pub fn load(path: &Path) -> Result<FileConfig, String> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
    let table = value.as_table().cloned().unwrap_or_default();
    let provider = table
        .get("provider")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    Ok(FileConfig {
        provider_type: provider
            .get("type")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        provider_model: provider
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        progress: table
            .get("progress")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

pub fn save(path: &Path, cfg: &FileConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut out = String::new();
    if let Some(p) = &cfg.progress {
        out.push_str(&format!("progress = \"{p}\"\n\n"));
    }
    if cfg.provider_type.is_some() || cfg.provider_model.is_some() {
        out.push_str("[provider]\n");
        if let Some(t) = &cfg.provider_type {
            out.push_str(&format!("type = \"{t}\"\n"));
        }
        if let Some(m) = &cfg.provider_model {
            out.push_str(&format!("model = \"{m}\"\n"));
        }
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
