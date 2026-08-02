//! Runtime API client and endpoint resolution.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vd_srv::api::{self, Endpoint, TransportKind};

use crate::config::GatewayConfig;
use crate::paths;

pub struct RuntimeClient {
    endpoint: Endpoint,
}

impl RuntimeClient {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }

    pub fn call(&self, method: &str, params: Option<Value>) -> Result<Value, String> {
        api::call(&self.endpoint, method, params).map_err(|e| e.to_string())
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}

pub fn resolve(
    config: &GatewayConfig,
    transport: Option<&str>,
    tcp: Option<&str>,
    socket: Option<&Path>,
) -> Result<Endpoint, String> {
    let transport = transport
        .map(str::to_string)
        .or_else(|| std::env::var(paths::ENV_TRANSPORT).ok())
        .or_else(|| config.transport.clone())
        .unwrap_or_else(|| "auto".into());
    let kind = TransportKind::parse(&transport)
        .ok_or_else(|| format!("unknown transport: {transport}"))?;
    let tcp = tcp
        .map(str::to_string)
        .or_else(|| std::env::var(paths::ENV_TCP).ok())
        .or_else(|| config.tcp.clone());
    let socket = socket
        .map(Path::to_path_buf)
        .or_else(|| std::env::var(paths::ENV_SOCKET).ok().map(PathBuf::from))
        .or_else(|| config.socket.clone());
    let data_dir = config
        .data_dir
        .clone()
        .unwrap_or_else(vd_srv::paths::data_dir);
    api::resolve_endpoint(kind, socket.as_deref(), None, tcp.as_deref(), &data_dir)
}
