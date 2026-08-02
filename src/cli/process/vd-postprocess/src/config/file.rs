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
    // Prefer [runner]; fall back to [provider].
    let runner = table
        .get("runner")
        .or_else(|| table.get("provider"))
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    Ok(FileConfig {
        runner_type: runner
            .get("type")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        runner_model: runner
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
    if cfg.runner_type.is_some() || cfg.runner_model.is_some() {
        out.push_str("[runner]\n");
        if let Some(t) = &cfg.runner_type {
            out.push_str(&format!("type = \"{t}\"\n"));
        }
        if let Some(m) = &cfg.runner_model {
            out.push_str(&format!("model = \"{m}\"\n"));
        }
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
