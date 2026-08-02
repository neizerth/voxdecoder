//! Write / remove VoxDecoder MCP server entries in AI client JSON configs.

use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::agents::{AgentAdapter, McpFormat};
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct McpServerSpec {
    pub command: String,
    pub args: Vec<String>,
    pub env: Map<String, Value>,
}

pub fn install_mcp(adapter: &AgentAdapter, spec: &McpServerSpec, dry_run: bool) -> Result<(), Error> {
    let Some(key) = adapter.mcp_format.key() else {
        return Ok(());
    };
    let Some(path) = adapter.preferred_config_path() else {
        return Err(Error::Message(format!(
            "{}: no MCP config path in adapter",
            adapter.name
        )));
    };

    if dry_run {
        eprintln!("  [dry-run] would install Bundle MCP in {}", path.display());
        return Ok(());
    }

    let mut root = read_json_object(&path)?;
    let servers = root
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let obj = servers
        .as_object_mut()
        .ok_or_else(|| Error::Message(format!("{}: `{key}` is not an object", path.display())))?;

    let mut entry = Map::new();
    entry.insert("command".into(), json!(spec.command));
    if !spec.args.is_empty() {
        entry.insert("args".into(), json!(spec.args));
    }
    if !spec.env.is_empty() {
        entry.insert("env".into(), Value::Object(spec.env.clone()));
    }
    obj.insert("voxdecoder".into(), Value::Object(entry));

    write_json_atomic(&path, &Value::Object(root))?;
    Ok(())
}

pub fn uninstall_mcp(adapter: &AgentAdapter, dry_run: bool) -> Result<(), Error> {
    if matches!(adapter.mcp_format, McpFormat::None) {
        return Ok(());
    }
    let Some(key) = adapter.mcp_format.key() else {
        return Ok(());
    };
    let Some(path) = adapter.preferred_config_path() else {
        return Ok(());
    };
    if !path.is_file() {
        return Ok(());
    }

    if dry_run {
        eprintln!("  [dry-run] would uninstall Bundle MCP from {}", path.display());
        return Ok(());
    }

    let mut root = read_json_object(&path)?;
    if let Some(Value::Object(servers)) = root.get_mut(key) {
        servers.remove("voxdecoder");
        servers.remove("vd-mcp");
    }
    write_json_atomic(&path, &Value::Object(root))?;
    Ok(())
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>, Error> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let raw = fs::read_to_string(path).map_err(|e| Error::Message(e.to_string()))?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value =
        serde_json::from_str(&raw).map_err(|e| Error::Message(format!("{}: {e}", path.display())))?;
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(Error::Message(format!(
            "{}: expected JSON object",
            path.display()
        ))),
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::Message(e.to_string()))?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|e| Error::Message(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, format!("{body}\n")).map_err(|e| Error::Message(e.to_string()))?;
    if path.exists() {
        let bak = path.with_extension("json.bak");
        let _ = fs::copy(path, bak);
    }
    fs::rename(&tmp, path).map_err(|e| Error::Message(e.to_string()))?;
    Ok(())
}
