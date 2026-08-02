//! JSON-RPC 2.0 wire types (framing is newline-delimited JSON).

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const JSONRPC_VERSION: &str = "2.0";

/// Standard + application JSON-RPC error codes.
pub mod code {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    /// Application / vd-srv domain errors.
    pub const APPLICATION: i64 = -32000;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
    Number(i64),
    String(String),
    Null,
}

impl Id {
    pub fn number(n: i64) -> Self {
        Self::Number(n)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

impl Request {
    pub fn call(id: Id, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            method: method.into(),
            params,
            id: Some(id),
        }
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorObject>,
    pub id: Option<Id>,
}

impl Response {
    pub fn success(id: Option<Id>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn failure(id: Option<Id>, error: ErrorObject) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorObject {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(code::METHOD_NOT_FOUND, format!("method not found: {method}"))
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(code::INVALID_PARAMS, msg)
    }

    pub fn application(msg: impl Into<String>) -> Self {
        Self::new(code::APPLICATION, msg)
    }

    pub fn parse(msg: impl Into<String>) -> Self {
        Self::new(code::PARSE_ERROR, msg)
    }

    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::new(code::INVALID_REQUEST, msg)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Notification {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            method: method.into(),
            params,
        }
    }
}

/// Outbound framed message.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Outbound {
    Response(Response),
    #[allow(dead_code)]
    Notification(Notification),
}
