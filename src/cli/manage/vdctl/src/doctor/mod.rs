//! Platform doctor.

use serde_json::json;

use crate::client;
use crate::error::Error;
use crate::resolve::Platform;

pub fn run(platform: &Platform, json: bool) -> Result<(), Error> {
    let runtime_bin = platform.vd_srv().is_file();
    let mcp_bin = platform.vd_mcp().is_file();
    let running = client::reachable(platform);
    let mut checks = vec![
        check("Runtime binary", runtime_bin),
        check("MCP binary", mcp_bin),
        check("Runtime reachable", running),
        check(
            "Data dir writable",
            is_writable_dir(&platform.data_dir),
        ),
    ];

    let mut api_version = None;
    let mut runtime_version = None;
    if running {
        if let Ok(info) = client::call(platform, "server.info", None) {
            api_version = info.get("api_version").and_then(|v| v.as_str()).map(str::to_string);
            runtime_version = info.get("version").and_then(|v| v.as_str()).map(str::to_string);
            checks.push(check("Runtime API", true));
        } else {
            checks.push(check("Runtime API", false));
        }
    }

    let ok = checks.iter().all(|c| c["status"] == "ok");
    let value = json!({
        "status": if ok { "ok" } else { "degraded" },
        "mode": platform.mode.as_str(),
        "cli_version": env!("CARGO_PKG_VERSION"),
        "runtime_version": runtime_version,
        "api_version": api_version,
        "checks": checks,
    });

    crate::output::emit_value(json, value.clone(), |v| {
        println!("Status      {}", v["status"].as_str().unwrap_or(""));
        println!("Mode        {}", platform.mode.as_str());
        println!("CLI         {}", env!("CARGO_PKG_VERSION"));
        if let Some(ver) = v["runtime_version"].as_str() {
            println!("Runtime     {ver}");
        }
        if let Some(ver) = v["api_version"].as_str() {
            println!("API         {ver}");
        }
        println!();
        if let Some(items) = v["checks"].as_array() {
            for c in items {
                let mark = if c["status"] == "ok" { "✓" } else { "✗" };
                println!("{mark} {}", c["name"].as_str().unwrap_or(""));
            }
        }
    })?;

    if ok {
        Ok(())
    } else {
        Err(Error::Message("doctor: one or more checks failed".into()))
    }
}

fn check(name: &str, ok: bool) -> serde_json::Value {
    json!({
        "name": name,
        "status": if ok { "ok" } else { "fail" }
    })
}

fn is_writable_dir(path: &std::path::Path) -> bool {
    if let Err(e) = std::fs::create_dir_all(path) {
        let _ = e;
        return false;
    }
    let probe = path.join(".vdctl-write-probe");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}
