//! JSON-RPC client over a transport connection.

use std::io::{BufRead, Write};

use serde_json::Value;

use super::rpc::{Id, Request, Response};
use super::transport::{connect, Duplex, Endpoint};

#[derive(Debug)]
pub struct RpcError {
    pub message: String,
    pub code: Option<i64>,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(c) = self.code {
            write!(f, "RPC error {c}: {}", self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl From<String> for RpcError {
    fn from(message: String) -> Self {
        Self {
            message,
            code: None,
        }
    }
}

/// Persistent JSON-RPC client on one duplex connection.
pub struct JsonRpcClient {
    duplex: Duplex,
    next_id: i64,
}

impl JsonRpcClient {
    pub fn connect(endpoint: &Endpoint) -> Result<Self, RpcError> {
        Ok(Self {
            duplex: connect(endpoint).map_err(RpcError::from)?,
            next_id: 1,
        })
    }

    pub fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, RpcError> {
        let id = Id::number(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let req = Request::call(id.clone(), method, params);
        let out = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        writeln!(self.duplex.writer, "{out}").map_err(|e| e.to_string())?;
        self.duplex.writer.flush().map_err(|e| e.to_string())?;

        loop {
            let mut line = String::new();
            let n = self
                .duplex
                .reader
                .read_line(&mut line)
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err(RpcError::from("connection closed".to_string()));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Skip notifications (have method, no result/error/id pair as response).
            if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                if v.get("method").is_some() && v.get("id").is_none() {
                    continue;
                }
            }

            let resp: Response = serde_json::from_str(trimmed).map_err(|e| {
                RpcError::from(format!("unexpected frame from server: {e}: {trimmed}"))
            })?;
            if let Some(ref rid) = resp.id {
                if !ids_equal(rid, &id) {
                    continue;
                }
            }
            if let Some(err) = resp.error {
                return Err(RpcError {
                    message: err.message,
                    code: Some(err.code),
                });
            }
            return Ok(resp.result.unwrap_or(Value::Null));
        }
    }
}

fn ids_equal(a: &Id, b: &Id) -> bool {
    match (a, b) {
        (Id::Number(x), Id::Number(y)) => x == y,
        (Id::String(x), Id::String(y)) => x == y,
        (Id::Null, Id::Null) => true,
        _ => false,
    }
}

/// One-shot RPC call (connect → call → drop).
pub fn call(
    endpoint: &Endpoint,
    method: &str,
    params: Option<Value>,
) -> Result<Value, RpcError> {
    let mut client = JsonRpcClient::connect(endpoint)?;
    client.call(method, params)
}
