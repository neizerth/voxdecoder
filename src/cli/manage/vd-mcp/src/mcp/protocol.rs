//! Minimal MCP JSON-RPC protocol handling.

use serde_json::{json, Value};

use crate::client::RuntimeClient;

use super::tools;

pub fn handle(client: &RuntimeClient, message: Value) -> Option<Value> {
    let id = message.get("id")?.clone();
    let method = message.get("method")?.as_str()?;
    let result: Result<Value, String> = (|| match method {
        "initialize" => Ok(json!({
            "protocolVersion": message
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2024-11-05"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "vd-mcp", "version": env!("CARGO_PKG_VERSION")}
        })),
        "tools/list" => Ok(json!({"tools": tools::list()})),
        "tools/call" => {
            let params = message
                .get("params")
                .ok_or_else(|| "tools/call params required".to_string())?;
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "tool name required".to_string())?;
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            tools::call(client, name, arguments).map(|data| {
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string())}]})
            })
        }
        "ping" => Ok(json!({})),
        _ => Err(format!("method not found: {method}")),
    })();
    Some(match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err(message) => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":message}}),
    })
}
