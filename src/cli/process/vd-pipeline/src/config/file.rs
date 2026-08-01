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
    Ok(FileConfig {
        progress: table
            .get("progress")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        asr: table
            .get("asr")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        continue_on_error: table.get("continue_on_error").and_then(|v| {
            v.as_bool().or_else(|| {
                v.as_str().and_then(|s| match s {
                    "on" | "true" | "1" => Some(true),
                    "off" | "false" | "0" => Some(false),
                    _ => None,
                })
            })
        }),
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
    if let Some(a) = &cfg.asr {
        out.push_str(&format!("asr = \"{a}\"\n"));
    }
    if let Some(c) = cfg.continue_on_error {
        out.push_str(&format!(
            "continue_on_error = \"{}\"\n",
            if c { "on" } else { "off" }
        ));
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
