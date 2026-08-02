//! Optional Runtime transports discovery (ADR 0007).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransportStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uds: Option<TransportEndpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp: Option<TransportEndpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<TransportEndpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grpc: Option<TransportEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportEndpoint {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

impl TransportEndpoint {
    pub fn on(endpoint: impl Into<String>) -> Self {
        Self {
            enabled: true,
            endpoint: Some(endpoint.into()),
        }
    }

    pub fn off() -> Self {
        Self {
            enabled: false,
            endpoint: None,
        }
    }
}
