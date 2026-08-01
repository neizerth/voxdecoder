//! Local model asset packs (install / list / remove).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backend::known_providers;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub provider: String,
    pub version: u32,
    pub status: String,
    #[serde(default)]
    pub notes: String,
}

pub fn provider_dir(provider: &str) -> PathBuf {
    paths::assets_root().join(provider)
}

pub fn manifest_path(provider: &str) -> PathBuf {
    provider_dir(provider).join("manifest.toml")
}

pub fn is_installed(provider: &str) -> bool {
    if provider == "stub" {
        return true;
    }
    manifest_path(provider).is_file()
}

pub fn install(provider: &str) -> Result<PathBuf, String> {
    if !known_providers().contains(&provider) {
        return Err(format!("unknown provider: {provider}"));
    }
    if provider == "stub" {
        // Always available; still write a marker for list/info consistency.
    }
    let dir = provider_dir(provider);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let status = if provider == "stub" {
        "ready"
    } else {
        "assets_registered"
    };
    let manifest = AssetManifest {
        provider: provider.into(),
        version: 1,
        status: status.into(),
        notes: if provider == "stub" {
            "Deterministic stub; no heavy assets.".into()
        } else {
            "Asset pack registered. Local inference runtime for this provider lands in a follow-up.".into()
        },
    };
    let text = toml_manifest(&manifest)?;
    fs::write(manifest_path(provider), text).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn remove(provider: &str) -> Result<(), String> {
    if provider == "stub" {
        // Keep stub usable; remove marker if present.
        let dir = provider_dir(provider);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let dir = provider_dir(provider);
    if !dir.exists() {
        return Err(format!("provider not installed: {provider}"));
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

pub fn list_installed() -> Vec<String> {
    let mut out = vec!["stub".into()];
    let root = paths::assets_root();
    if let Ok(entries) = fs::read_dir(&root) {
        for ent in entries.flatten() {
            let name = ent.file_name().to_string_lossy().into_owned();
            if name == "stub" {
                continue;
            }
            if ent.path().join("manifest.toml").is_file() {
                out.push(name);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn info(provider: &str) -> Result<AssetManifest, String> {
    if provider == "stub" && !manifest_path(provider).is_file() {
        return Ok(AssetManifest {
            provider: "stub".into(),
            version: 1,
            status: "ready".into(),
            notes: "Built-in deterministic backend.".into(),
        });
    }
    let path = manifest_path(provider);
    if !path.is_file() {
        return Err(format!("provider not installed: {provider}"));
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    parse_manifest(&text)
}

fn toml_manifest(m: &AssetManifest) -> Result<String, String> {
    let mut s = String::new();
    s.push_str(&format!("provider = \"{}\"\n", m.provider));
    s.push_str(&format!("version = {}\n", m.version));
    s.push_str(&format!("status = \"{}\"\n", m.status));
    s.push_str(&format!("notes = \"{}\"\n", escape_toml(&m.notes)));
    Ok(s)
}

fn parse_manifest(text: &str) -> Result<AssetManifest, String> {
    let value: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
    let table = value.as_table().cloned().unwrap_or_default();
    Ok(AssetManifest {
        provider: table
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .into(),
        version: table
            .get("version")
            .and_then(toml::Value::as_integer)
            .unwrap_or(1) as u32,
        status: table
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .into(),
        notes: table
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .into(),
    })
}

fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn assets_root_display() -> PathBuf {
    paths::assets_root()
}

pub fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    Ok(())
}
