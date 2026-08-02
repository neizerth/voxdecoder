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
    /// Optional HTTP transport (ADR 0006). Disabled by default.
    #[serde(default)]
    pub http: HttpConfig,
    /// Optional gRPC transport (ADR 0007). Disabled by default.
    #[serde(default)]
    pub grpc: GrpcConfig,
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

/// HTTP transport config (`[http]` in toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_http_bind")]
    pub bind: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: default_http_bind(),
        }
    }
}

impl HttpConfig {
    /// CLI `--http ADDR` wins; else enabled config bind.
    pub fn listen_addr(&self, cli: Option<&str>) -> Option<String> {
        if let Some(addr) = cli {
            let t = addr.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
        if self.enabled {
            Some(self.bind.clone())
        } else {
            None
        }
    }
}

/// gRPC transport config (`[grpc]` in toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_grpc_bind")]
    pub bind: String,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: default_grpc_bind(),
        }
    }
}

impl GrpcConfig {
    /// CLI `--grpc ADDR` wins; else enabled config bind.
    pub fn listen_addr(&self, cli: Option<&str>) -> Option<String> {
        if let Some(addr) = cli {
            let t = addr.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
        if self.enabled {
            Some(self.bind.clone())
        } else {
            None
        }
    }
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
        #[cfg(target_os = "macos")]
        {
            // Single Metal context — concurrent gigaam Metal loads OOM the GPU.
            resource_classes.insert(
                "metal_gpu".into(),
                ResourceClassConfig { capacity: 1 },
            );
        }
        Self {
            workers: default_workers(),
            resource_classes,
            http: HttpConfig::default(),
            grpc: GrpcConfig::default(),
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
fn default_http_bind() -> String {
    "127.0.0.1:7701".into()
}
fn default_grpc_bind() -> String {
    "127.0.0.1:7702".into()
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
    let mut raw: ServerConfig = toml::from_str(&body).map_err(|e| e.to_string())?;
    ensure_default_resource_classes(&mut raw);
    Ok(FileConfig { raw })
}

/// Fill platform Resource Classes missing from older configs (do not overwrite explicit values).
pub fn ensure_default_resource_classes(cfg: &mut ServerConfig) {
    cfg.resource_classes.entry("cpu".into()).or_insert_with(|| {
        ResourceClassConfig {
            capacity: default_cpu_capacity(),
        }
    });
    #[cfg(target_os = "macos")]
    {
        cfg.resource_classes
            .entry("metal_gpu".into())
            .or_insert(ResourceClassConfig { capacity: 1 });
    }
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
