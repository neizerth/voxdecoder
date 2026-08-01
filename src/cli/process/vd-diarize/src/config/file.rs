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
    let backend = table
        .get("backend")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    Ok(FileConfig {
        provider: backend
            .get("provider")
            .and_then(|v| v.as_str())
            .or_else(|| table.get("provider").and_then(|v| v.as_str()))
            .map(str::to_string),
        model: backend
            .get("model")
            .and_then(|v| v.as_str())
            .or_else(|| table.get("model").and_then(|v| v.as_str()))
            .map(str::to_string),
        progress: table
            .get("progress")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        device: table
            .get("device")
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
        out.push_str(&format!("progress = \"{p}\"\n"));
    }
    if let Some(d) = &cfg.device {
        out.push_str(&format!("device = \"{d}\"\n"));
    }
    if cfg.provider.is_some() || cfg.model.is_some() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("[backend]\n");
        if let Some(p) = &cfg.provider {
            out.push_str(&format!("provider = \"{p}\"\n"));
        }
        if let Some(m) = &cfg.model {
            out.push_str(&format!("model = \"{m}\"\n"));
        }
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
