//! Gateway configuration.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default)]
    pub transport: Option<String>,
    #[serde(default)]
    pub tcp: Option<String>,
    #[serde(default)]
    pub socket: Option<PathBuf>,
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
}

pub fn load(path: &Path) -> Result<GatewayConfig, String> {
    if !path.exists() {
        return Ok(GatewayConfig::default());
    }
    toml::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn save(path: &Path, config: &GatewayConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        path,
        toml::to_string_pretty(config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
