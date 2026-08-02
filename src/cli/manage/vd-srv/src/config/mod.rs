//! Server + Job configuration.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_workers")]
    pub workers: u32,
    #[serde(default)]
    pub resource_classes: BTreeMap<String, ResourceClassConfig>,
    #[serde(default)]
    pub http: Option<String>,
    /// Optional TCP listen address (`127.0.0.1:7701`). Enables TCP transport when set.
    #[serde(default)]
    pub tcp: Option<String>,
    /// Windows named pipe path (ignored on Unix until pipe transport ships).
    #[serde(default)]
    pub pipe: Option<String>,
    /// `auto` | `uds` | `pipe` | `tcp`
    #[serde(default)]
    pub transport: crate::api::TransportKind,
    #[serde(default)]
    pub socket: Option<PathBuf>,
    #[serde(default)]
    pub retention: RetentionConfig,
    #[serde(default = "default_history")]
    pub history: u32,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceClassConfig {
    pub capacity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    #[serde(default = "default_artifacts_ttl")]
    pub artifacts: String,
    #[serde(default = "default_logs_ttl")]
    pub logs: String,
    #[serde(default = "default_events_ttl")]
    pub events: String,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            artifacts: default_artifacts_ttl(),
            logs: default_logs_ttl(),
            events: default_events_ttl(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        let mut resource_classes = BTreeMap::new();
        resource_classes.insert(
            "cpu".into(),
            ResourceClassConfig {
                capacity: default_cpu_capacity(),
            },
        );
        Self {
            workers: default_workers(),
            resource_classes,
            http: None,
            tcp: None,
            pipe: None,
            transport: crate::api::TransportKind::Auto,
            socket: None,
            retention: RetentionConfig::default(),
            history: default_history(),
            log_level: default_log_level(),
            data_dir: None,
        }
    }
}

fn default_workers() -> u32 {
    1
}
fn default_history() -> u32 {
    100
}
fn default_log_level() -> String {
    "info".into()
}
fn default_artifacts_ttl() -> String {
    "30d".into()
}
fn default_logs_ttl() -> String {
    "14d".into()
}
fn default_events_ttl() -> String {
    "forever".into()
}
fn default_cpu_capacity() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

#[derive(Debug, Clone, Default)]
pub struct FileConfig {
    pub raw: ServerConfig,
}

pub fn load(path: &Path) -> Result<FileConfig, String> {
    if !path.exists() {
        return Ok(FileConfig {
            raw: ServerConfig::default(),
        });
    }
    let body = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: ServerConfig = toml::from_str(&body).map_err(|e| e.to_string())?;
    Ok(FileConfig { raw })
}

pub fn save(path: &Path, cfg: &ServerConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = toml::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, body).map_err(|e| e.to_string())
}

pub fn effective_data_dir(cfg: &ServerConfig, override_dir: Option<&Path>) -> PathBuf {
    override_dir
        .map(Path::to_path_buf)
        .or_else(|| cfg.data_dir.clone())
        .unwrap_or_else(paths::data_dir)
}

pub fn effective_socket(cfg: &ServerConfig, data: &Path) -> PathBuf {
    cfg.socket
        .clone()
        .unwrap_or_else(|| paths::default_socket_path(data))
}

pub fn effective_endpoint(
    cfg: &ServerConfig,
    data: &Path,
    transport_override: Option<crate::api::TransportKind>,
    socket_override: Option<&Path>,
    tcp_override: Option<&str>,
) -> Result<crate::api::Endpoint, String> {
    crate::api::resolve_endpoint(
        transport_override.unwrap_or(cfg.transport),
        socket_override.or(cfg.socket.as_deref()),
        cfg.pipe.as_deref(),
        tcp_override.or(cfg.tcp.as_deref()),
        data,
    )
}

pub mod defaults {
    pub use super::ServerConfig;
}
