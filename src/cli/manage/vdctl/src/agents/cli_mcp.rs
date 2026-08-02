//! CLI MCP registration (e.g. Claude Code `claude mcp add/remove/list`).

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::agents::{AgentAdapter, McpServerSpec};
use crate::error::Error;

const DEFAULT_SERVER: &str = "voxdecoder";

pub fn server_name(adapter: &AgentAdapter) -> &str {
    adapter
        .mcp_server_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SERVER)
}

pub fn cli_bin(adapter: &AgentAdapter) -> Result<String, Error> {
    let resolved = adapter.resolve_for_host();
    resolved
        .bins
        .first()
        .cloned()
        .or_else(|| adapter.bins.first().cloned())
        .ok_or_else(|| {
            Error::Message(format!(
                "{}: CLI adapter has no `bins` entry",
                adapter.name
            ))
        })
}

/// Prefer config-file presence — `claude mcp list` health-checks servers and can hang.
pub fn is_registered(adapter: &AgentAdapter) -> bool {
    let name = server_name(adapter);
    for path in adapter
        .resolve_for_host()
        .config_paths
        .iter()
        .map(|p| crate::agents::expand_path(p))
    {
        if path.is_file() && config_has_server(&path, name) {
            return true;
        }
    }
    false
}

fn config_has_server(path: &Path, name: &str) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return raw.to_ascii_lowercase().contains(&name.to_ascii_lowercase());
    };
    if value.pointer(&format!("/mcpServers/{name}")).is_some() {
        return true;
    }
    if let Some(projects) = value.get("projects").and_then(|p| p.as_object()) {
        for project in projects.values() {
            if project
                .pointer(&format!("/mcpServers/{name}"))
                .is_some()
            {
                return true;
            }
        }
    }
    false
}

pub fn install(adapter: &AgentAdapter, spec: &McpServerSpec, dry_run: bool) -> Result<(), Error> {
    use std::io::{self, Write};

    let bin = cli_bin(adapter)?;
    let name = server_name(adapter).to_string();

    if dry_run {
        println!(
            "    · [dry-run] {bin} mcp add {name} -s {} -- {}",
            adapter.mcp_scope.as_deref().unwrap_or("user"),
            spec.command
        );
        let _ = io::stdout().flush();
        return Ok(());
    }

    println!("    · {bin} mcp remove {name}…");
    let _ = io::stdout().flush();
    let mut remove = Command::new(&bin);
    remove.args(["mcp", "remove", &name]);
    if let Some(scope) = adapter.mcp_scope.as_deref() {
        remove.args(["-s", scope]);
    }
    let _ = remove.output();

    println!("    · {bin} mcp add {name} (stdio → {})…", spec.command);
    let _ = io::stdout().flush();

    let mut cmd = Command::new(&bin);
    cmd.arg("mcp").arg("add").arg(&name);
    if let Some(scope) = adapter.mcp_scope.as_deref() {
        cmd.args(["-s", scope]);
    }
    for (key, value) in &spec.env {
        let Some(v) = value.as_str() else {
            continue;
        };
        cmd.arg("-e").arg(format!("{key}={v}"));
    }
    cmd.arg("--").arg(&spec.command);
    for a in &spec.args {
        cmd.arg(a);
    }

    let output = cmd
        .output()
        .map_err(|e| Error::Message(format!("failed to run `{bin} mcp add`: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        return Err(Error::Message(format!(
            "`{bin} mcp add {name}` failed: {} {}",
            err.trim(),
            out.trim()
        )));
    }
    Ok(())
}

pub fn uninstall(adapter: &AgentAdapter, dry_run: bool) -> Result<(), Error> {
    let bin = cli_bin(adapter)?;
    let name = server_name(adapter).to_string();

    if dry_run {
        eprintln!("  [dry-run] would run: {bin} mcp remove {name}");
        return Ok(());
    }

    let mut cmd = Command::new(&bin);
    cmd.args(["mcp", "remove", &name]);
    if let Some(scope) = adapter.mcp_scope.as_deref() {
        cmd.args(["-s", scope]);
    }
    let output = cmd
        .output()
        .map_err(|e| Error::Message(format!("failed to run `{bin} mcp remove`: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        if err.contains("not found") || err.contains("does not exist") || err.contains("no mcp")
        {
            return Ok(());
        }
        return Err(Error::Message(format!(
            "`{bin} mcp remove {name}` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

pub fn list_output(adapter: &AgentAdapter) -> Result<String, Error> {
    for path in adapter
        .resolve_for_host()
        .config_paths
        .iter()
        .map(|p| crate::agents::expand_path(p))
    {
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|e| Error::Message(e.to_string()))?;
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            if let Some(servers) = value.get("mcpServers") {
                return Ok(serde_json::to_string_pretty(servers).unwrap_or_default());
            }
        }
        return Ok(raw);
    }
    Ok(String::new())
}
