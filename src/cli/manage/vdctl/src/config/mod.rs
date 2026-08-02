//! Platform configuration (`vdctl.toml`).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutoBuild {
    Always,
    #[default]
    Missing,
    Never,
}

impl AutoBuild {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "always" => Some(Self::Always),
            "missing" => Some(Self::Missing),
            "never" => Some(Self::Never),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Missing => "missing",
            Self::Never => "never",
        }
    }
}

/// Workspace Cargo profile for Runtime binaries (`target/debug` vs `target/release`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuildProfile {
    #[default]
    Debug,
    Release,
}

impl BuildProfile {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "debug" | "dev" => Some(Self::Debug),
            "release" | "prod" => Some(Self::Release),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    pub fn target_dir_name(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<PathBuf>,
    /// Workspace binary profile: `debug` (default) or `release` / `prod`.
    #[serde(default)]
    pub build: BuildProfile,
    #[serde(default)]
    pub auto_build: AutoBuild,
    #[serde(default)]
    pub auto_start_mcp: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp: Option<String>,
    /// HTTP transport bind for vd-srv (ADR 0006), e.g. `127.0.0.1:7701`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            workspace: None,
            build: BuildProfile::Debug,
            auto_build: AutoBuild::Missing,
            auto_start_mcp: false,
            transport: None,
            tcp: None,
            http: None,
            socket: None,
            data_dir: None,
        }
    }
}

pub fn load(path: &Path) -> Result<PlatformConfig, String> {
    if !path.exists() {
        return Ok(PlatformConfig::default());
    }
    toml::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn save(path: &Path, config: &PlatformConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        path,
        toml::to_string_pretty(config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn get(config: &PlatformConfig, key: &str) -> Option<String> {
    match key {
        "workspace" => config.workspace.as_ref().map(|p| p.display().to_string()),
        "build" => Some(config.build.as_str().to_string()),
        "auto_build" => Some(config.auto_build.as_str().to_string()),
        "auto_start_mcp" => Some(config.auto_start_mcp.to_string()),
        "transport" => config.transport.clone(),
        "tcp" => config.tcp.clone(),
        "http" => config.http.clone(),
        "socket" => config.socket.as_ref().map(|p| p.display().to_string()),
        "data_dir" => config.data_dir.as_ref().map(|p| p.display().to_string()),
        _ => None,
    }
}

pub fn set(config: &mut PlatformConfig, key: &str, value: &str) -> Result<(), String> {
    match key {
        "workspace" => {
            config.workspace = if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
        }
        "build" => {
            config.build = BuildProfile::parse(value).ok_or_else(|| {
                format!("invalid build: {value} (expected debug|dev|release|prod)")
            })?;
        }
        "auto_build" => {
            config.auto_build = AutoBuild::parse(value)
                .ok_or_else(|| format!("invalid auto_build: {value}"))?;
        }
        "auto_start_mcp" => {
            config.auto_start_mcp = parse_bool(value)?;
        }
        "transport" => config.transport = empty_to_none(value),
        "tcp" => config.tcp = empty_to_none(value),
        "http" => config.http = empty_to_none(value),
        "socket" => {
            config.socket = if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
        }
        "data_dir" => {
            config.data_dir = if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
        }
        other => return Err(format!("unknown config key: {other}")),
    }
    Ok(())
}

fn empty_to_none(value: &str) -> Option<String> {
    let t = value.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(format!("invalid bool: {other}")),
    }
}
