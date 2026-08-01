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
    let diarization = table
        .get("diarization")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let alignment = table
        .get("alignment")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    Ok(FileConfig {
        diarization_enabled: diarization
            .get("enabled")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        alignment_mode: alignment
            .get("mode")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        asr: table
            .get("asr")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        max_parallel: table
            .get("max_parallel")
            .and_then(toml::Value::as_integer)
            .map(|n| n as u32),
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
    if let Some(a) = &cfg.asr {
        out.push_str(&format!("asr = \"{a}\"\n"));
    }
    if let Some(n) = cfg.max_parallel {
        out.push_str(&format!("max_parallel = {n}\n"));
    }
    if cfg.diarization_enabled.is_some() {
        out.push_str("\n[diarization]\n");
        if let Some(e) = &cfg.diarization_enabled {
            out.push_str(&format!("enabled = \"{e}\"\n"));
        }
    }
    if cfg.alignment_mode.is_some() {
        out.push_str("\n[alignment]\n");
        if let Some(m) = &cfg.alignment_mode {
            out.push_str(&format!("mode = \"{m}\"\n"));
        }
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
