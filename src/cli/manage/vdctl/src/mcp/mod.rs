//! MCP gateway process + Bundle install into AI apps (ADR 0005).

mod bundle;

use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Map};

use crate::agents::{self, McpServerSpec};
use crate::client;
use crate::error::Error;
use crate::paths;
use crate::resolve::Platform;
use crate::skills;

pub use bundle::{bundle_installed, bundle_path, build as build_bundle};

fn say(msg: &str) {
    println!("{msg}");
    let _ = io::stdout().flush();
}

fn say_detail(msg: &str) {
    println!("    · {msg}");
    let _ = io::stdout().flush();
}

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

    say("Discovering Skills...");
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

    if !opts.no_skills {
        say(&format!(
            "Synchronizing Skills → {}",
            paths::skills_dir().display()
        ));
        if selected_skills.is_empty() {
            println!("  (none)");
        } else {
            let prune = opts.skills.is_none();
            skills::sync_to_home(platform, &selected_skills, prune, opts.dry_run)?;
        }
        println!();
    }

    say("Probing AI applications (desktop + CLI)…");
    let apps = agents::filter_installed_adapters(opts.apps.as_deref())?;
    if apps.is_empty() {
        return Err(Error::Message(
            "no installed AI applications matched".into(),
        ));
    }

    let needs_bundle = apps.iter().any(|a| a.kind.uses_bundle());

    println!();
    say("Detected:");
    println!();
    for a in &apps {
        println!("✔ {} ({})", a.name, a.kind.as_str());
    }
    println!();

    if needs_bundle {
        say("Building MCP Bundle (packaging/mcp → .mcpb)…");
        let bundle = bundle::build(platform, opts.dry_run)?;
        if !opts.dry_run {
            println!("✔ {}", bundle.display());
        }
        println!();
    }

    let mut env = Map::new();
    env.insert("VD_TRANSPORT".into(), json!(platform.transport));
    env.insert(
        "VD_SOCKET".into(),
        json!(platform.socket.display().to_string()),
    );
    if let Some(http) = &platform.http {
        env.insert("VD_HTTP".into(), json!(http));
    }
    let spec = McpServerSpec {
        command: mcp_bin.display().to_string(),
        args: vec![],
        env,
    };

    say("Installing integrations…");
    println!();
    let total = apps.len();
    for (i, app) in apps.iter().enumerate() {
        say(&format!("[{}/{}] {}", i + 1, total, app.name));
        if app.has_cli_mcp() {
            match agents::install_cli_mcp(app, &spec, opts.dry_run) {
                Ok(()) => println!("    ✔ MCP registered (CLI)"),
                Err(e) => println!("    ✘ MCP register ({e})"),
            }
        } else if app.kind.uses_bundle() {
            if app.mcp_format.supports_json_mcp() {
                say_detail("writing MCP / Bundle config…");
            }
            match agents::install_mcp(app, &spec, opts.dry_run) {
                Ok(()) => {
                    if app.mcp_format.supports_json_mcp() {
                        println!("    ✔ Bundle installed");
                    } else {
                        println!("    · Bundle (MCP format unsupported — skipped)");
                    }
                }
                Err(e) => {
                    if app.mcp_format.supports_json_mcp() {
                        println!("    ✘ Bundle ({e})");
                    } else {
                        println!("    · Bundle (MCP format unsupported — skipped)");
                    }
                }
            }
        } else {
            println!("    · no MCP installer for this adapter yet");
        }
        if !opts.no_skills {
            for skill in &selected_skills {
                say_detail(&format!("linking skill {}…", skill.id));
                match skills::link_skill_to_app(app, skill, opts.dry_run) {
                    Ok(()) => println!("    ✔ {}", skill.id),
                    Err(e) => println!("    ✘ {} ({e})", skill.id),
                }
            }
        }
        println!();
    }

    say("Verifying integrations…");
    println!();
    verify_inner(platform, opts, &selected_skills, &apps)?;
    Ok(())
}

pub fn update(platform: &Platform, opts: &InstallOpts) -> Result<(), Error> {
    println!("Updating MCP (synchronize Skills → rebuild Bundle → apps → verify)…");
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
            if app.has_cli_mcp() {
                match agents::uninstall_cli_mcp(app, opts.dry_run) {
                    Ok(()) => println!("    ✔ MCP unregistered (CLI)"),
                    Err(e) => println!("    ✘ MCP unregister ({e})"),
                }
            } else if app.kind.uses_bundle() {
                match agents::uninstall_mcp(app, opts.dry_run) {
                    Ok(()) => println!("    ✔ Bundle uninstalled"),
                    Err(e) => println!("    ✘ Bundle ({e})"),
                }
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
        let skills_n = v["skills_count"].as_u64().unwrap_or(0);
        if v["skills_installed"].as_bool().unwrap_or(false) {
            println!("Skills       ✔ ({skills_n} installed)");
        } else {
            println!("Skills       ✘");
        }
        if let Some(apps) = v["apps"].as_array() {
            println!();
            for a in apps {
                if !a["installed"].as_bool().unwrap_or(false) {
                    continue;
                }
                println!("{}", a["name"].as_str().unwrap_or(""));
                println!();
                if let Some(checks) = a["checks"].as_array() {
                    for c in checks {
                        let ok = c["ok"].as_bool().unwrap_or(false);
                        println!(
                            "    {} {}",
                            if ok { "✔" } else { "✘" },
                            c["label"].as_str().unwrap_or("")
                        );
                    }
                }
                if let Some(hint) = a["hint"].as_str() {
                    if !a["ok"].as_bool().unwrap_or(true) {
                        println!();
                        for line in hint.lines() {
                            println!("    {line}");
                        }
                    }
                }
                println!();
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
    for app in apps {
        say(&app.name);
        println!();
        if app.has_cli_mcp() {
            say_detail("checking CLI on PATH…");
            let cli_ok = which_ok(app) || opts.dry_run;
            say_detail("checking MCP registration in config…");
            let reg_ok = opts.dry_run || agents::cli_mcp_registered(app);
            println!("    {} CLI found", if cli_ok { "✔" } else { "✘" });
            println!(
                "    {} MCP registered",
                if reg_ok { "✔" } else { "✘" }
            );
            if reg_ok {
                println!("    ✔ Server visible");
            } else if !opts.dry_run {
                println!("    ✘ Server visible");
                println!();
                println!(
                    "    VoxDecoder MCP server is not registered in {}.",
                    app.name
                );
                println!();
                println!("    Run:");
                println!();
                println!("        vdctl mcp install --apps {}", app.id);
            }
        } else if app.kind.uses_bundle() {
            say_detail("checking Bundle / MCP config…");
            let installed = opts.dry_run
                || agents::discover_agents()
                    .iter()
                    .find(|a| a.id == app.id)
                    .map(|a| a.configured)
                    .unwrap_or(false)
                || !app.mcp_format.supports_json_mcp();
            if app.mcp_format.supports_json_mcp() {
                println!(
                    "    {} Bundle installed",
                    if installed { "✔" } else { "✘" }
                );
                println!(
                    "    {} Bundle active",
                    if installed { "✔" } else { "✘" }
                );
            } else {
                println!("    · Bundle (unsupported)");
            }
        }
        if !opts.no_skills {
            for skill in selected_skills {
                let ok = opts.dry_run || skills::skill_linked_in_app(app, &skill.id);
                println!("    {} {}", if ok { "✔" } else { "✘" }, skill.id);
            }
        }
        println!();
    }
    if client::reachable(platform) {
        println!("✔ Runtime reachable");
    } else {
        println!("· Runtime not running (start with `vdctl ensure`)");
    }
    println!();
    println!("Verification complete.");
    Ok(())
}

fn which_ok(app: &agents::AgentAdapter) -> bool {
    app.resolve_for_host().bins.iter().any(|b| {
        let (cmd, arg) = if cfg!(windows) {
            ("where", b.as_str())
        } else {
            ("which", b.as_str())
        };
        Command::new(cmd)
            .arg(arg)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

fn verify_report(platform: &Platform) -> Result<serde_json::Value, Error> {
    let gateway = platform.vd_mcp().is_file();
    let bundle = bundle_installed();
    let runtime = client::reachable(platform);
    let skills_home = paths::skills_dir();
    let skills_count = fs::read_dir(&skills_home)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir() && e.path().join("skill.md").is_file())
                .count()
        })
        .unwrap_or(0);
    let skills_ok = skills_count > 0;

    let mut issues = Vec::new();
    if !gateway {
        issues.push("vd-mcp binary missing".into());
    }

    let bundle_meta = bundle::read_bundle_manifest().ok().flatten();
    let bundle_version = bundle_meta
        .as_ref()
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let adapters = agents::adapters();
    let discovered = agents::discover_agents();
    let mut apps = Vec::new();
    let mut integration_ok = true;

    for agent in &discovered {
        if !agent.installed {
            continue;
        }
        let adapter = adapters.iter().find(|a| a.id == agent.id);
        let mut checks = Vec::new();
        let mut ok = true;
        let mut hint = None;

        if let Some(ad) = adapter {
            if ad.has_cli_mcp() {
                let cli = which_ok(ad);
                checks.push(json!({"label": "CLI found", "ok": cli}));
                let reg = agent.configured;
                checks.push(json!({"label": "MCP registered", "ok": reg}));
                checks.push(json!({"label": "Server visible", "ok": reg}));
                ok = cli && reg;
                if !reg {
                    hint = Some(format!(
                        "VoxDecoder MCP server is not registered in {}.\n\nRun:\n\n    vdctl mcp install --apps {}",
                        ad.name, ad.id
                    ));
                    integration_ok = false;
                    issues.push(format!("{}: MCP not registered", ad.name));
                }
            } else if ad.kind.uses_bundle() {
                if ad.mcp_format.supports_json_mcp() {
                    checks.push(json!({"label": "Bundle installed", "ok": agent.configured}));
                    checks.push(json!({"label": "Bundle active", "ok": agent.configured}));
                    ok = agent.configured;
                    if !agent.configured {
                        hint = Some(format!(
                            "Bundle not active in {}.\n\nRun:\n\n    vdctl mcp install --apps {}",
                            ad.name, ad.id
                        ));
                        // Desktop not configured is a soft fail for overall ok only if we require all apps
                    }
                } else {
                    checks.push(json!({"label": "Bundle (unsupported)", "ok": true}));
                }
            } else {
                checks.push(json!({"label": "detected", "ok": true}));
            }
        }

        apps.push(json!({
            "id": agent.id,
            "name": agent.name,
            "kind": agent.kind,
            "installed": agent.installed,
            "configured": agent.configured,
            "ok": ok,
            "checks": checks,
            "hint": hint,
        }));
    }

    if !skills_ok {
        issues.push(format!(
            "no Skills under {} (run `vdctl mcp install`)",
            skills_home.display()
        ));
    }

    // Overall: gateway required; CLI integrations that are installed must be registered.
    let ok = gateway && integration_ok;

    Ok(json!({
        "ok": ok,
        "bundle_installed": bundle,
        "bundle_path": bundle_path().display().to_string(),
        "bundle_version": bundle_version,
        "gateway_installed": gateway,
        "gateway_bin": platform.vd_mcp().display().to_string(),
        "runtime_reachable": runtime,
        "skills_installed": skills_ok,
        "skills_count": skills_count,
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
