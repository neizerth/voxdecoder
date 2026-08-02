//! Discover / inspect / info snapshots.

use serde_json::json;

use crate::client;
use crate::error::Error;
use crate::paths;
use crate::resolve::Platform;

pub fn info(platform: &Platform, json: bool) -> Result<(), Error> {
    let running = client::reachable(platform);
    let mut runtime_version = None;
    let mut api_version = None;
    if running {
        if let Ok(info) = client::call(platform, "server.info", None) {
            runtime_version = info.get("version").cloned();
            api_version = info.get("api_version").cloned();
        }
    }
    let value = json!({
        "platform": std::env::consts::OS,
        "mode": platform.mode.as_str(),
        "runtime_version": runtime_version,
        "api_version": api_version,
        "transport": platform.transport,
        "socket": platform.socket.display().to_string(),
        "data_dir": platform.data_dir.display().to_string(),
        "running": running,
        "cli_version": env!("CARGO_PKG_VERSION"),
    });
    crate::output::emit_value(json, value, |v| {
        println!("Platform    {}", v["platform"].as_str().unwrap_or(""));
        println!("Mode        {}", v["mode"].as_str().unwrap_or(""));
        println!(
            "Runtime     {}",
            if running { "Running" } else { "Stopped" }
        );
        println!("Transport   {}", v["transport"].as_str().unwrap_or(""));
        println!("Socket      {}", v["socket"].as_str().unwrap_or(""));
        println!("Data        {}", v["data_dir"].as_str().unwrap_or(""));
        println!("CLI         {}", v["cli_version"].as_str().unwrap_or(""));
    })
}

pub fn paths_cmd(platform: &Platform, json: bool) -> Result<(), Error> {
    let value = json!({
        "config": paths::config_path().display().to_string(),
        "home": paths::home_dir().display().to_string(),
        "data": platform.data_dir.display().to_string(),
        "models": paths::models_dir().display().to_string(),
        "socket": platform.socket.display().to_string(),
        "bin_dir": platform.bin_dir.display().to_string(),
        "workspace": platform.workspace.as_ref().map(|p| p.display().to_string()),
    });
    crate::output::emit_value(json, value, |v| {
        for (k, label) in [
            ("config", "Config"),
            ("home", "Home"),
            ("data", "Data"),
            ("models", "Models"),
            ("socket", "Socket"),
            ("bin_dir", "Bin"),
        ] {
            println!("{label:<10} {}", v[k]);
        }
        if let Some(ws) = v["workspace"].as_str() {
            println!("{:<10} {ws}", "Workspace");
        }
    })
}

pub fn env_cmd(platform: &Platform, export: bool) -> Result<(), Error> {
    let pairs = [
        ("VD_HOME", paths::home_dir().display().to_string()),
        ("VD_TRANSPORT", platform.transport.clone()),
        ("VD_SOCKET", platform.socket.display().to_string()),
        ("VD_MODELS_DIR", paths::models_dir().display().to_string()),
        ("VDCTL_CONFIG", paths::config_path().display().to_string()),
    ];
    if export {
        for (k, v) in pairs {
            println!("{k}={v}");
        }
        if let Some(tcp) = &platform.tcp {
            println!("VD_TCP={tcp}");
        }
        if let Some(http) = &platform.http {
            println!("VD_HTTP={http}");
        }
    } else {
        for (k, v) in pairs {
            println!("{k:<16} {v}");
        }
        if let Some(tcp) = &platform.tcp {
            println!("{:<16} {tcp}", "VD_TCP");
        }
        if let Some(http) = &platform.http {
            println!("{:<16} {http}", "VD_HTTP");
        }
    }
    Ok(())
}

pub fn discover(platform: &Platform, json: bool) -> Result<(), Error> {
    let running = client::reachable(platform);
    let agents = crate::agents::discover_agents();
    let skills = crate::skills::discover(platform);
    let value = json!({
        "mode": platform.mode.as_str(),
        "runtime": {
            "installed": platform.vd_srv().is_file(),
            "running": running,
            "bin": platform.vd_srv().display().to_string(),
        },
        "mcp": {
            "installed": platform.vd_mcp().is_file(),
            "bin": platform.vd_mcp().display().to_string(),
        },
        "agents": agents,
        "applications": agents,
        "skills": skills,
        "models": [],
        "assets": [],
        "transports": [platform.transport],
        "accelerators": [],
    });
    crate::output::emit_value(json, value, |_| {
        println!("Applications");
        println!();
        crate::agents::print_agents_human(&crate::agents::discover_agents());
        println!("Skills");
        println!();
        let report = crate::skills::discover(platform);
        if report.skills.is_empty() {
            println!("(none under {})", report.root);
        } else {
            for s in &report.skills {
                let mark = if s.valid { "✔" } else { "✘" };
                println!("{mark} {}", s.id);
            }
        }
        for d in &report.diagnostics {
            eprintln!("! {d}");
        }
    })
}

pub fn inspect(platform: &Platform, json: bool) -> Result<(), Error> {
    // Full snapshot ≈ discover + paths + versions for Desktop cold start.
    let running = client::reachable(platform);
    let mut server_info = json!(null);
    if running {
        if let Ok(info) = client::call(platform, "server.info", None) {
            server_info = info;
        }
    }
    let agents = crate::agents::discover_agents();
    let skills = crate::skills::discover(platform);
    let value = json!({
        "mode": platform.mode.as_str(),
        "cli_version": env!("CARGO_PKG_VERSION"),
        "runtime": {
            "installed": platform.vd_srv().is_file(),
            "running": running,
            "bin": platform.vd_srv().display().to_string(),
            "server_info": server_info,
        },
        "mcp": {
            "installed": platform.vd_mcp().is_file(),
            "bin": platform.vd_mcp().display().to_string(),
        },
        "paths": {
            "config": paths::config_path().display().to_string(),
            "home": paths::home_dir().display().to_string(),
            "data": platform.data_dir.display().to_string(),
            "models": paths::models_dir().display().to_string(),
            "socket": platform.socket.display().to_string(),
            "skills": crate::skills::skills_root(platform).display().to_string(),
        },
        "agents": agents,
        "skills": skills,
        "models": [],
        "assets": [],
        "config": {},
        "diagnostics": {},
    });
    crate::output::emit_value(json, value, |_| {
        println!("Use --json for the full inspect snapshot.");
        let _ = discover(platform, false);
    })
}

pub fn health(platform: &Platform, json: bool) -> Result<(), Error> {
    if !client::reachable(platform) {
        return Err(Error::NotReachable(
            "Runtime not running (try `vdctl up`)".into(),
        ));
    }
    let health = client::call(platform, "server.health", None)?;
    crate::output::emit_value(json, health.clone(), |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    })
}
