//! MCP process lifecycle and host registration.

use std::fs;
use std::process::{Command, Stdio};

use serde_json::{json, Map};

use crate::agents::{self, McpServerSpec};
use crate::error::Error;
use crate::paths;
use crate::resolve::Platform;
use crate::skills;

#[derive(Debug, Clone, Default)]
pub struct RegisterOpts {
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

pub fn status(platform: &Platform, json: bool) -> Result<(), Error> {
    let running = status_running(platform);
    let value = json!({
        "running": running,
        "bin": platform.vd_mcp().display().to_string(),
        "installed": platform.vd_mcp().is_file(),
    });
    crate::output::emit_value(json, value, |v| {
        println!(
            "MCP         {}",
            if v["running"].as_bool().unwrap_or(false) {
                "Running"
            } else if v["installed"].as_bool().unwrap_or(false) {
                "Stopped"
            } else {
                "Not installed"
            }
        );
    })
}

pub fn register(platform: &Platform, opts: &RegisterOpts) -> Result<(), Error> {
    let mcp_bin = platform.vd_mcp();
    if !mcp_bin.is_file() {
        return Err(Error::Message(format!(
            "vd-mcp not found at {}",
            mcp_bin.display()
        )));
    }

    let report = skills::discover(platform);
    let selected_skills = if opts.no_skills {
        Vec::new()
    } else {
        skills::select_skills(&report, opts.skills.as_deref(), &opts.exclude)?
    };
    let apps = agents::filter_installed_adapters(opts.apps.as_deref())?;

    if apps.is_empty() {
        return Err(Error::Message(
            "no installed AI applications matched".into(),
        ));
    }

    println!("Discovering Skills...");
    println!();
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

    println!("Installing MCP...");
    println!();
    for app in &apps {
        println!("{}", app.name);
        match agents::install_mcp(app, &spec, opts.dry_run) {
            Ok(()) => println!("    ✔ Runtime"),
            Err(e) => {
                if app.mcp_format.supports_json_mcp() {
                    println!("    ✘ Runtime ({e})");
                } else {
                    println!("    · Runtime (MCP format unsupported — skipped)");
                }
            }
        }
        if !opts.no_skills {
            for skill in &selected_skills {
                match skills::install_skill(app, skill, opts.dry_run) {
                    Ok(()) => println!("    ✔ {}", skill.id),
                    Err(e) => println!("    ✘ {} ({e})", skill.id),
                }
            }
        }
        println!();
    }

    println!("Verification...");
    println!();
    let agents_now = agents::discover_agents();
    for app in &apps {
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
        println!("{}  MCP {mcp_mark}", app.name);
        if !opts.no_skills {
            for skill in &selected_skills {
                let ok = opts.dry_run || skills::skill_installed(app, &skill.id);
                println!("         {} {}", if ok { "✔" } else { "✘" }, skill.id);
            }
        }
    }
    println!();
    println!("Verification complete.");
    Ok(())
}

pub fn unregister(_platform: &Platform, opts: &RegisterOpts) -> Result<(), Error> {
    let apps = agents::filter_installed_adapters(opts.apps.as_deref())?;
    if apps.is_empty() {
        return Err(Error::Message(
            "no installed AI applications matched".into(),
        ));
    }

    let report = skills::discover(_platform);
    // --skills / --exclude → only those skills, keep MCP
    // --no-skills → MCP only
    // default → MCP + all discovered skills
    let skills_only = opts.skills.is_some() || !opts.exclude.is_empty();
    let selected_skills = if opts.no_skills {
        Vec::new()
    } else if skills_only {
        skills::select_skills(&report, opts.skills.as_deref(), &opts.exclude)?
    } else {
        report.skills.clone()
    };
    let strip_mcp = !skills_only;

    for app in &apps {
        println!("{}", app.name);
        if strip_mcp {
            match agents::uninstall_mcp(app, opts.dry_run) {
                Ok(()) => println!("    ✔ MCP unregistered"),
                Err(e) => println!("    ✘ MCP ({e})"),
            }
        }
        for skill in &selected_skills {
            match skills::remove_skill(app, &skill.id, opts.dry_run) {
                Ok(()) => println!("    ✔ removed {}", skill.id),
                Err(e) => println!("    ✘ {} ({e})", skill.id),
            }
        }
        println!();
    }
    Ok(())
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
                "registered": a.configured,
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
    });
    crate::output::emit_value(json, value, |_| {
        agents::print_agents_human(&agents);
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
