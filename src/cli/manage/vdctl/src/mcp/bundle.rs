//! MCP Bundle (`.mcpb`) builder — ADR 0005.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::error::Error;
use crate::paths;
use crate::resolve::Platform;

const BUNDLE_NAME: &str = "voxdecoder.mcpb";

pub fn packaging_mcp_dir(platform: &Platform) -> Option<PathBuf> {
    platform.workspace.as_ref().map(|ws| ws.join("packaging/mcp"))
}

pub fn bundle_path() -> PathBuf {
    paths::mcp_bundle_path()
}

/// Build `$VD_HOME/bundles/voxdecoder.mcpb` from `packaging/mcp` + local `vd-mcp`.
pub fn build(platform: &Platform, dry_run: bool) -> Result<PathBuf, Error> {
    let out = bundle_path();
    let mcp_bin = platform.vd_mcp();
    if !mcp_bin.is_file() {
        return Err(Error::Message(format!(
            "vd-mcp not found at {} (build the gateway first)",
            mcp_bin.display()
        )));
    }

    let packaging = packaging_mcp_dir(platform)
        .filter(|p| p.is_dir())
        .ok_or_else(|| {
            Error::Message(
                "packaging/mcp not found (run from a Workspace or ship packaging with the install)"
                    .into(),
            )
        })?;

    let manifest_path = packaging.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(Error::Message(format!(
            "missing {}",
            manifest_path.display()
        )));
    }

    let mut manifest: Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path).map_err(|e| Error::Message(e.to_string()))?,
    )
    .map_err(|e| Error::Message(e.to_string()))?;

    if let Some(obj) = manifest.as_object_mut() {
        obj.insert(
            "built_at".into(),
            json!(chrono_like_now()),
        );
        let mut server = obj
            .get("server")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .as_object()
            .cloned()
            .unwrap_or_default();
        let mut env = serde_json::Map::new();
        env.insert("VD_TRANSPORT".into(), json!(platform.transport));
        env.insert(
            "VD_SOCKET".into(),
            json!(platform.socket.display().to_string()),
        );
        if let Some(http) = &platform.http {
            env.insert("VD_HTTP".into(), json!(http));
        }
        server.insert(
            "mcp_config".into(),
            json!({
                "command": mcp_bin.display().to_string(),
                "args": [],
                "env": env,
            }),
        );
        obj.insert("server".into(), Value::Object(server));
    }

    if dry_run {
        eprintln!(
            "[dry-run] would build {} from {}",
            out.display(),
            packaging.display()
        );
        return Ok(out);
    }

    fs::create_dir_all(paths::bundles_dir()).map_err(|e| Error::Message(e.to_string()))?;
    if out.exists() {
        fs::remove_file(&out).map_err(|e| Error::Message(e.to_string()))?;
    }

    let file = fs::File::create(&out).map_err(|e| Error::Message(e.to_string()))?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest_body =
        serde_json::to_string_pretty(&manifest).map_err(|e| Error::Message(e.to_string()))?;
    zip.start_file("manifest.json", opts)
        .map_err(|e| Error::Message(e.to_string()))?;
    zip.write_all(manifest_body.as_bytes())
        .map_err(|e| Error::Message(e.to_string()))?;
    zip.write_all(b"\n")
        .map_err(|e| Error::Message(e.to_string()))?;

    let icon = packaging.join("icon.png");
    if icon.is_file() {
        let bytes = fs::read(&icon).map_err(|e| Error::Message(e.to_string()))?;
        zip.start_file("icon.png", opts)
            .map_err(|e| Error::Message(e.to_string()))?;
        zip.write_all(&bytes)
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    // Record gateway binary path for installers (not embedded — keeps Bundle small).
    let gateway = json!({
        "vd_mcp": mcp_bin.display().to_string(),
        "bundle": BUNDLE_NAME,
    });
    zip.start_file("gateway.json", opts)
        .map_err(|e| Error::Message(e.to_string()))?;
    zip.write_all(
        serde_json::to_string_pretty(&gateway)
            .map_err(|e| Error::Message(e.to_string()))?
            .as_bytes(),
    )
    .map_err(|e| Error::Message(e.to_string()))?;
    zip.write_all(b"\n")
        .map_err(|e| Error::Message(e.to_string()))?;

    zip.finish().map_err(|e| Error::Message(e.to_string()))?;
    Ok(out)
}

pub fn bundle_installed() -> bool {
    bundle_path().is_file()
}

pub fn read_bundle_manifest() -> Result<Option<Value>, Error> {
    let path = bundle_path();
    if !path.is_file() {
        return Ok(None);
    }
    let file = fs::File::open(&path).map_err(|e| Error::Message(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| Error::Message(e.to_string()))?;
    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|e| Error::Message(e.to_string()))?;
    let mut raw = String::new();
    std::io::Read::read_to_string(&mut entry, &mut raw)
        .map_err(|e| Error::Message(e.to_string()))?;
    let value = serde_json::from_str(&raw).map_err(|e| Error::Message(e.to_string()))?;
    Ok(Some(value))
}

fn chrono_like_now() -> String {
    // Avoid chrono dep: RFC3339-ish via system time seconds.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_path_under_home() {
        assert!(bundle_path().ends_with("voxdecoder.mcpb"));
    }
}
