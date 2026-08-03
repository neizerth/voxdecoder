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

    obj.insert("voxdecoder".into(), server_entry(adapter.mcp_format, spec));

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

fn server_entry(format: McpFormat, spec: &McpServerSpec) -> Value {
    match format {
        McpFormat::Mcp => {
            // OpenCode: { type, command: [bin, ...args], environment, enabled }
            let mut command = vec![json!(spec.command)];
            for a in &spec.args {
                command.push(json!(a));
            }
            let mut entry = Map::new();
            entry.insert("type".into(), json!("local"));
            entry.insert("command".into(), Value::Array(command));
            entry.insert("enabled".into(), json!(true));
            if !spec.env.is_empty() {
                entry.insert("environment".into(), Value::Object(spec.env.clone()));
            }
            Value::Object(entry)
        }
        McpFormat::Stdio => {
            // Crush: { type: stdio, command, args?, env? }
            let mut entry = Map::new();
            entry.insert("type".into(), json!("stdio"));
            entry.insert("command".into(), json!(spec.command));
            if !spec.args.is_empty() {
                entry.insert("args".into(), json!(spec.args));
            }
            if !spec.env.is_empty() {
                entry.insert("env".into(), Value::Object(spec.env.clone()));
            }
            Value::Object(entry)
        }
        _ => {
            // Cursor / Claude Desktop / VS Code / Gemini: { command, args?, env? }
            let mut entry = Map::new();
            entry.insert("command".into(), json!(spec.command));
            if !spec.args.is_empty() {
                entry.insert("args".into(), json!(spec.args));
            }
            if !spec.env.is_empty() {
                entry.insert("env".into(), Value::Object(spec.env.clone()));
            }
            Value::Object(entry)
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AppKind;

    fn adapter(format: McpFormat, config: &str) -> AgentAdapter {
        AgentAdapter {
            id: "t".into(),
            name: "T".into(),
            kind: AppKind::Cli,
            mcp_format: format,
            app_paths: vec![],
            marker_dirs: vec![],
            bins: vec!["opencode".into()],
            config_paths: vec![config.into()],
            skill_dirs: vec![],
            mcp_server_name: None,
            mcp_scope: None,
            configured_markers: vec![],
            macos: None,
            linux: None,
            windows: None,
        }
    }

    #[test]
    fn writes_opencode_local_mcp_shape() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("opencode.json");
        let ad = adapter(McpFormat::Mcp, cfg.to_str().unwrap());
        let mut env = Map::new();
        env.insert("VD_TRANSPORT".into(), json!("uds"));
        let spec = McpServerSpec {
            command: "/opt/vd-mcp".into(),
            args: vec!["serve".into()],
            env,
        };
        install_mcp(&ad, &spec, false).unwrap();
        let raw = fs::read_to_string(&cfg).unwrap();
        let v: Value = serde_json::from_str(&raw).unwrap();
        let entry = &v["mcp"]["voxdecoder"];
        assert_eq!(entry["type"], json!("local"));
        assert_eq!(entry["enabled"], json!(true));
        assert_eq!(entry["command"], json!(["/opt/vd-mcp", "serve"]));
        assert_eq!(entry["environment"]["VD_TRANSPORT"], json!("uds"));
        assert!(entry.get("args").is_none());
        assert!(entry.get("env").is_none());
    }

    #[test]
    fn writes_cursor_style_mcp_servers() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("mcp.json");
        let ad = adapter(McpFormat::McpServers, cfg.to_str().unwrap());
        let spec = McpServerSpec {
            command: "/opt/vd-mcp".into(),
            args: vec![],
            env: Map::new(),
        };
        install_mcp(&ad, &spec, false).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        let entry = &v["mcpServers"]["voxdecoder"];
        assert_eq!(entry["command"], json!("/opt/vd-mcp"));
        assert!(entry.get("type").is_none());
    }

    #[test]
    fn writes_crush_stdio_mcp_shape() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("crush.json");
        let ad = adapter(McpFormat::Stdio, cfg.to_str().unwrap());
        let mut env = Map::new();
        env.insert("VD_SOCKET".into(), json!("/tmp/vd.sock"));
        let spec = McpServerSpec {
            command: "/opt/vd-mcp".into(),
            args: vec![],
            env,
        };
        install_mcp(&ad, &spec, false).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        let entry = &v["mcp"]["voxdecoder"];
        assert_eq!(entry["type"], json!("stdio"));
        assert_eq!(entry["command"], json!("/opt/vd-mcp"));
        assert_eq!(entry["env"]["VD_SOCKET"], json!("/tmp/vd.sock"));
        assert!(entry.get("args").is_none());
        assert!(entry.get("environment").is_none());
        assert!(entry.get("enabled").is_none());
    }

    #[test]
    fn writes_gemini_style_mcp_servers() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("settings.json");
        let ad = adapter(McpFormat::McpServers, cfg.to_str().unwrap());
        let mut env = Map::new();
        env.insert("VD_TRANSPORT".into(), json!("uds"));
        let spec = McpServerSpec {
            command: "/opt/vd-mcp".into(),
            args: vec![],
            env,
        };
        install_mcp(&ad, &spec, false).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        let entry = &v["mcpServers"]["voxdecoder"];
        assert_eq!(entry["command"], json!("/opt/vd-mcp"));
        assert_eq!(entry["env"]["VD_TRANSPORT"], json!("uds"));
        assert!(entry.get("type").is_none());
    }

    #[test]
    fn uninstall_removes_opencode_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("opencode.json");
        let ad = adapter(McpFormat::Mcp, cfg.to_str().unwrap());
        let spec = McpServerSpec {
            command: "/opt/vd-mcp".into(),
            args: vec![],
            env: Map::new(),
        };
        install_mcp(&ad, &spec, false).unwrap();
        uninstall_mcp(&ad, false).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert!(v["mcp"].get("voxdecoder").is_none());
    }
}
