//! Platform config and data paths.

use std::path::PathBuf;

use directories::ProjectDirs;

pub const ENV_CONFIG: &str = "VDCTL_CONFIG";
pub const ENV_TRANSPORT: &str = "VD_TRANSPORT";
pub const ENV_TCP: &str = "VD_TCP";
pub const ENV_SOCKET: &str = "VD_SOCKET";
pub const ENV_HOME: &str = "VD_HOME";
pub const ENV_MODELS_DIR: &str = "VD_MODELS_DIR";

const APP: &str = "voxdecoder";

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var(ENV_CONFIG) {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    ProjectDirs::from("", "", APP)
        .map(|d| d.config_dir().join("vdctl.toml"))
        .unwrap_or_else(|| PathBuf::from("vdctl.toml"))
}

/// AI client adapters (`agents.toml`). Override next to `vdctl.toml`; else built-in.
pub fn agents_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("VDCTL_AGENTS") {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    config_path()
        .parent()
        .map(|p| p.join("agents.toml"))
        .unwrap_or_else(|| PathBuf::from("agents.toml"))
}

/// Platform home (data root for installed layout).
pub fn home_dir() -> PathBuf {
    if let Ok(p) = std::env::var(ENV_HOME) {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    ProjectDirs::from("", "", APP)
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".voxdecoder"))
}

pub fn models_dir() -> PathBuf {
    if let Ok(p) = std::env::var(ENV_MODELS_DIR) {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    home_dir().join("models")
}

/// Installed Skills root (`$VD_HOME/skills`).
pub fn skills_dir() -> PathBuf {
    home_dir().join("skills")
}

/// Built MCP Bundles (`$VD_HOME/bundles`).
pub fn bundles_dir() -> PathBuf {
    home_dir().join("bundles")
}

pub fn mcp_bundle_path() -> PathBuf {
    bundles_dir().join("voxdecoder.mcpb")
}

pub fn runtime_data_dir() -> PathBuf {
    vd_srv::paths::data_dir()
}

pub fn runtime_pid_path(data: &std::path::Path) -> PathBuf {
    data.join("vdctl").join("runtime.pid")
}

pub fn mcp_pid_path(data: &std::path::Path) -> PathBuf {
    data.join("vdctl").join("mcp.pid")
}

pub fn runtime_log_path(data: &std::path::Path) -> PathBuf {
    data.join("vdctl").join("runtime.log")
}
