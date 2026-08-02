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
        .and_then(|v| {
            v.as_str()
                .map(str::to_string)
                .or_else(|| v.as_table()?.get("type")?.as_str().map(str::to_string))
        });
    Ok(FileConfig {
        provider,
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
        out.push_str(&format!("progress = \"{p}\"\n"));
    }
    if let Some(t) = &cfg.provider {
        out.push_str(&format!("provider = \"{t}\"\n"));
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
