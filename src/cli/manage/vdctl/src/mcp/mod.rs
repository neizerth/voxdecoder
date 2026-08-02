//! MCP gateway process + Bundle install into AI apps (ADR 0005).

mod bundle;

use std::fs;
use std::process::{Command, Stdio};

use serde_json::{json, Map};

use crate::agents::{self, McpServerSpec};
use crate::client;
use crate::error::Error;
use crate::paths;
use crate::resolve::Platform;
use crate::skills;

pub use bundle::{bundle_installed, bundle_path, build as build_bundle};

#[derive(Debug, Clone, Default)]
pub struct InstallOpts {
    pub apps: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub exclude: Vec<String>,
    pub no_skills: bool,
    pub dry_run: bool,
}

pub fn start(platform: &Platform) -> Result<(), Error> {
    let bin = platform.vd_mcp();
    if !bin.is_file() {
        return Err(Error::Message(format!(
            "vd-mcp not found at {}",
            bin.display()
        )));
    }
    if status_running(platform) {
        eprintln!("MCP already running");
        return Ok(());
    }

    fs::create_dir_all(platform.data_dir.join("vdctl")).map_err(|e| Error::Message(e.to_string()))?;
    let log = platform.data_dir.join("vdctl").join("mcp.log");
    let out = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .map_err(|e| Error::Message(e.to_string()))?;
    let err = out.try_clone().map_err(|e| Error::Message(e.to_string()))?;

    let child = Command::new(&bin)
        .args([
            "serve",
            "--transport",
            "uds",
            "--socket",
            platform.socket.to_str().unwrap_or(""),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| Error::Message(format!("failed to start vd-mcp: {e}")))?;

    fs::write(
        paths::mcp_pid_path(&platform.data_dir),
        child.id().to_string(),
    )
    .map_err(|e| Error::Message(e.to_string()))?;
    eprintln!("MCP started (pid {})", child.id());
    Ok(())
}

pub fn stop(platform: &Platform) -> Result<(), Error> {
    let path = paths::mcp_pid_path(&platform.data_dir);
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(pid) = raw.trim().parse::<u32>() {
            #[cfg(unix)]
            {
                let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).status();
            }
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .status();
            }
            let _ = fs::remove_file(&path);
            eprintln!("MCP stopped");
            return Ok(());
        }
    }
    Ok(())
}

pub fn restart(platform: &Platform) -> Result<(), Error> {
    stop(platform)?;
    start(platform)
}

pub fn build(platform: &Platform, dry_run: bool) -> Result<(), Error> {
    let path = bundle::build(platform, dry_run)?;
    if !dry_run {
        println!("Built {}", path.display());
    }
    Ok(())
}

/// Sync Skills → build Bundle → install into AI apps → verify.
pub fn install(platform: &Platform, opts: &InstallOpts) -> Result<(), Error> {
    let mcp_bin = platform.vd_mcp();
    if !mcp_bin.is_file() {
        return Err(Error::Message(format!(
            "vd-mcp not found at {}",
            mcp_bin.display()
        )));
    }

    println!("Discovering Skills...");
    println!();
    let report = skills::discover(platform);
    let selected_skills = if opts.no_skills {
        Vec::new()
    } else {
        skills::select_skills(&report, opts.skills.as_deref(), &opts.exclude)?
    };
    if selected_skills.is_empty() {
        if opts.no_skills {
            println!("(skills disabled)");
        } else {
            println!("(none)");
        }
    } else {
        for s in &selected_skills {
            println!("✔ {}", s.id);
        }
    }
    for d in &report.diagnostics {
        eprintln!("! {d}");
    }
    println!();

    if !opts.no_skills && !selected_skills.is_empty() {
        println!("Installing Skills → {}", paths::skills_dir().display());
        skills::sync_to_home(platform, &selected_skills, opts.dry_run)?;
        println!();
    }

    println!("Building MCP Bundle...");
    let bundle = bundle::build(platform, opts.dry_run)?;
    if !opts.dry_run {
        println!("✔ {}", bundle.display());
    }
    println!();

    let apps = agents::filter_installed_adapters(opts.apps.as_deref())?;
    if apps.is_empty() {
        return Err(Error::Message(
            "no installed AI applications matched".into(),
        ));
    }

    println!("Detecting AI applications...");
    println!();
    for a in &apps {
        println!("✔ {}", a.name);
    }
    println!();

    let mut env = Map::new();
    env.insert("VD_TRANSPORT".into(), json!(platform.transport));
    env.insert(
        "VD_SOCKET".into(),
        json!(platform.socket.display().to_string()),
    );
    let spec = McpServerSpec {
        command: mcp_bin.display().to_string(),
        args: vec![],
        env,
    };

    println!("Installing Bundle...");
    println!();
    for app in &apps {
        println!("{}", app.name);
        match agents::install_mcp(app, &spec, opts.dry_run) {
            Ok(()) => println!("    ✔ Bundle / Gateway"),
            Err(e) => {
                if app.mcp_format.supports_json_mcp() {
                    println!("    ✘ Bundle ({e})");
                } else {
                    println!("    · Bundle (MCP format unsupported — skipped)");
                }
            }
        }
        if !opts.no_skills {
            for skill in &selected_skills {
                match skills::link_skill_to_app(app, skill, opts.dry_run) {
                    Ok(()) => println!("    ✔ {}", skill.id),
                    Err(e) => println!("    ✘ {} ({e})", skill.id),
                }
            }
        }
        println!();
    }

    verify_inner(platform, opts, &selected_skills, &apps)?;
    Ok(())
}

pub fn update(platform: &Platform, opts: &InstallOpts) -> Result<(), Error> {
    println!("Updating MCP Bundle + Skills...");
    println!();
    install(platform, opts)
}

pub fn uninstall(_platform: &Platform, opts: &InstallOpts) -> Result<(), Error> {
    let apps = agents::filter_installed_adapters(opts.apps.as_deref())?;
    if apps.is_empty() {
        return Err(Error::Message(
            "no installed AI applications matched".into(),
        ));
    }

    let report = skills::discover(_platform);
    let skills_only = opts.skills.is_some() || !opts.exclude.is_empty();
    let selected_skills = if opts.no_skills {
        Vec::new()
    } else if skills_only {
        skills::select_skills(&report, opts.skills.as_deref(), &opts.exclude)?
    } else {
        report.skills.clone()
    };
    let strip_bundle = !skills_only;

    for app in &apps {
        println!("{}", app.name);
        if strip_bundle {
            match agents::uninstall_mcp(app, opts.dry_run) {
                Ok(()) => println!("    ✔ Bundle uninstalled"),
                Err(e) => println!("    ✘ Bundle ({e})"),
            }
        }
        for skill in &selected_skills {
            match skills::unlink_skill_from_app(app, &skill.id, opts.dry_run) {
                Ok(()) => println!("    ✔ removed {}", skill.id),
                Err(e) => println!("    ✘ {} ({e})", skill.id),
            }
        }
        println!();
    }
    Ok(())
}

pub fn verify(platform: &Platform, json: bool) -> Result<(), Error> {
    let report = verify_report(platform)?;
    crate::output::emit_value(json, report.clone(), |v| {
        println!("Bundle       {}", check_mark(v["bundle_installed"].as_bool()));
        println!("Gateway bin  {}", check_mark(v["gateway_installed"].as_bool()));
        println!("Runtime      {}", check_mark(v["runtime_reachable"].as_bool()));
        println!(
            "Skills home  {}",
            check_mark(v["skills_installed"].as_bool())
        );
        if let Some(apps) = v["apps"].as_array() {
            println!();
            for a in apps {
                let mark = if a["bundle_configured"].as_bool().unwrap_or(false) {
                    "✔"
                } else if a["installed"].as_bool().unwrap_or(false) {
                    "✘"
                } else {
                    "·"
                };
                println!(
                    "{mark} {}",
                    a["name"].as_str().unwrap_or(a["id"].as_str().unwrap_or(""))
                );
            }
        }
        if let Some(issues) = v["issues"].as_array() {
            for i in issues {
                if let Some(msg) = i.as_str() {
                    eprintln!("! {msg}");
                }
            }
        }
    })?;
    if report.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        Ok(())
    } else {
        Err(Error::Message("mcp verify failed".into()))
    }
}

fn verify_inner(
    platform: &Platform,
    opts: &InstallOpts,
    selected_skills: &[skills::Skill],
    apps: &[agents::AgentAdapter],
) -> Result<(), Error> {
    println!("Verification...");
    println!();
    let agents_now = agents::discover_agents();
    for app in apps {
        let configured = agents_now
            .iter()
            .find(|a| a.id == app.id)
            .map(|a| a.configured)
            .unwrap_or(false);
        let mcp_mark = if app.mcp_format.supports_json_mcp() {
            if configured || opts.dry_run {
                "✔"
            } else {
                "✘"
            }
        } else {
            "·"
        };
        println!("{}  Bundle {mcp_mark}", app.name);
        if !opts.no_skills {
            for skill in selected_skills {
                let ok = opts.dry_run || skills::skill_linked_in_app(app, &skill.id);
                println!("         {} {}", if ok { "✔" } else { "✘" }, skill.id);
            }
        }
    }
    println!();
    if bundle_installed() || opts.dry_run {
        println!("✔ Bundle artifact");
    } else {
        println!("✘ Bundle artifact missing");
    }
    if client::reachable(platform) {
        println!("✔ Runtime reachable");
    } else {
        println!("· Runtime not running (start with `vdctl up`)");
    }
    println!();
    println!("Verification complete.");
    Ok(())
}

fn verify_report(platform: &Platform) -> Result<serde_json::Value, Error> {
    let gateway = platform.vd_mcp().is_file();
    let bundle = bundle_installed();
    let runtime = client::reachable(platform);
    let skills_home = paths::skills_dir();
    let skills_ok = skills_home.is_dir()
        && fs::read_dir(&skills_home)
            .map(|rd| {
                rd.flatten().any(|e| {
                    e.path().is_dir() && e.path().join("skill.md").is_file()
                })
            })
            .unwrap_or(false);

    let mut issues = Vec::new();
    if !gateway {
        issues.push("vd-mcp binary missing".into());
    }
    if !bundle {
        issues.push("voxdecoder.mcpb missing (run `vdctl mcp build`)".into());
    }
    if !skills_ok {
        issues.push(format!(
            "no Skills under {} (run `vdctl mcp install`)",
            skills_home.display()
        ));
    }

    let bundle_meta = bundle::read_bundle_manifest().ok().flatten();
    let bundle_version = bundle_meta
        .as_ref()
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let agents = agents::discover_agents();
    let apps: Vec<_> = agents
        .iter()
        .map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "installed": a.installed,
                "bundle_configured": a.configured,
            })
        })
        .collect();

    let ok = gateway && bundle;

    Ok(json!({
        "ok": ok,
        "bundle_installed": bundle,
        "bundle_path": bundle_path().display().to_string(),
        "bundle_version": bundle_version,
        "gateway_installed": gateway,
        "gateway_bin": platform.vd_mcp().display().to_string(),
        "runtime_reachable": runtime,
        "skills_installed": skills_ok,
        "skills_dir": skills_home.display().to_string(),
        "apps": apps,
        "issues": issues,
    }))
}

pub fn status(platform: &Platform, json: bool) -> Result<(), Error> {
    let running = status_running(platform);
    let report = verify_report(platform)?;
    let value = json!({
        "process_running": running,
        "bin": platform.vd_mcp().display().to_string(),
        "installed": platform.vd_mcp().is_file(),
        "bundle": {
            "path": bundle_path().display().to_string(),
            "installed": bundle_installed(),
        },
        "verify": report,
    });
    crate::output::emit_value(json, value, |v| {
        println!(
            "Gateway     {}",
            if v["process_running"].as_bool().unwrap_or(false) {
                "Running"
            } else if v["installed"].as_bool().unwrap_or(false) {
                "Stopped"
            } else {
                "Not installed"
            }
        );
        println!(
            "Bundle      {}",
            if v["bundle"]["installed"].as_bool().unwrap_or(false) {
                "installed"
            } else {
                "missing"
            }
        );
        println!("Path        {}", v["bundle"]["path"].as_str().unwrap_or(""));
    })
}

pub fn list(platform: &Platform, json: bool) -> Result<(), Error> {
    let agents = agents::discover_agents();
    let hosts: Vec<_> = agents
        .iter()
        .map(|a| {
            json!({
                "id": a.id,
                "name": a.name,
                "installed": a.installed,
                "bundle_configured": a.configured,
                "config_path": a.config_path,
                "mcp_format": a.mcp_format,
                "skill_dirs": a.skill_dirs,
            })
        })
        .collect();
    let value = json!({
        "hosts": hosts,
        "mcp_bin": platform.vd_mcp().display().to_string(),
        "mcp_installed": platform.vd_mcp().is_file(),
        "bundle_path": bundle_path().display().to_string(),
        "bundle_installed": bundle_installed(),
    });
    crate::output::emit_value(json, value, |_| {
        agents::print_agents_human(&agents);
        println!();
        println!(
            "Bundle  {}",
            if bundle_installed() {
                bundle_path().display().to_string()
            } else {
                "not built".into()
            }
        );
    })
}

fn status_running(platform: &Platform) -> bool {
    let path = paths::mcp_pid_path(&platform.data_dir);
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(pid) = raw.trim().parse::<u32>() else {
        return false;
    };
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        let _ = pid;
        true
    }
}

fn check_mark(v: Option<bool>) -> &'static str {
    if v.unwrap_or(false) {
        "✔"
    } else {
        "✘"
    }
}

/// Parse comma-separated CLI lists (`claude,cursor`).
pub fn parse_csv_list(raw: Option<&str>) -> Option<Vec<String>> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
    .filter(|v: &Vec<String>| !v.is_empty())
}
