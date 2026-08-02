//! Runtime API client (Operator surface).

use serde_json::Value;
use vd_srv::api::{self, Endpoint, TransportKind};

use crate::error::Error;
use crate::resolve::Platform;

pub fn endpoint(platform: &Platform) -> Result<Endpoint, Error> {
    let kind = TransportKind::parse(&platform.transport)
        .ok_or_else(|| Error::Usage(format!("unknown transport: {}", platform.transport)))?;
    api::resolve_endpoint(
        kind,
        Some(platform.socket.as_path()),
        None,
        platform.tcp.as_deref(),
        &platform.data_dir,
    )
    .map_err(Error::Message)
}

pub fn call(platform: &Platform, method: &str, params: Option<Value>) -> Result<Value, Error> {
    let ep = endpoint(platform)?;
    api::call(&ep, method, params).map_err(|e| Error::NotReachable(e.to_string()))
}

pub fn ping(platform: &Platform) -> Result<Value, Error> {
    call(platform, "server.ping", None)
}

pub fn reachable(platform: &Platform) -> bool {
    ping(platform).is_ok()
}
