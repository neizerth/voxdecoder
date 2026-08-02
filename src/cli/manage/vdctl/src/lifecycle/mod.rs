//! Runtime process lifecycle (`up` / `down` / …).

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::client;
use crate::config::PlatformConfig;
use crate::error::Error;
use crate::paths;
use crate::resolve::{self, Platform};

pub fn up(platform: &Platform, config: &PlatformConfig) -> Result<(), Error> {
    if client::reachable(platform) {
        eprintln!("Runtime already running");
        return Ok(());
    }
    start_runtime(platform, config)
}

/// Ensure Runtime is ready: no-op if already running, otherwise start it.
pub fn ensure(platform: &Platform, config: &PlatformConfig) -> Result<(), Error> {
    if client::reachable(platform) {
        eprintln!("Runtime ready");
        return Ok(());
    }
    eprintln!("Runtime not running — starting…");
    start_runtime(platform, config)
}

fn start_runtime(platform: &Platform, config: &PlatformConfig) -> Result<(), Error> {
    resolve::ensure_runtime_built(platform, config.auto_build, platform.build)?;
    let bin = platform.vd_srv();
    if !bin.is_file() {
        return Err(Error::Message(format!(
            "vd-srv not found at {}\nRun `cargo build -p vd-srv` (workspace) or `vdctl install`.",
            bin.display()
        )));
    }

    fs::create_dir_all(platform.data_dir.join("vdctl")).map_err(|e| Error::Message(e.to_string()))?;
    let log_path = paths::runtime_log_path(&platform.data_dir);
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| Error::Message(e.to_string()))?;
    let log_err = log.try_clone().map_err(|e| Error::Message(e.to_string()))?;

    let mut cmd = Command::new(&bin);
    cmd.arg("serve")
        .arg("--data-dir")
        .arg(&platform.data_dir)
        .arg("--socket")
        .arg(&platform.socket)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    if let Some(tcp) = &platform.tcp {
        cmd.arg("--tcp").arg(tcp);
    }
    if let Some(http) = &platform.http {
        cmd.arg("--http").arg(http);
    }

    let child = cmd
        .spawn()
        .map_err(|e| Error::Message(format!("failed to start vd-srv: {e}")))?;
    write_pid(&paths::runtime_pid_path(&platform.data_dir), child.id())?;

    wait_ready(platform, Duration::from_secs(30))?;
    eprintln!("Runtime started (pid {})", child.id());

    if config.auto_start_mcp {
        crate::mcp::start(platform)?;
    }
    Ok(())
}

pub fn down(platform: &Platform) -> Result<(), Error> {
    let _ = crate::mcp::stop(platform);

    if client::reachable(platform) {
        let _ = client::call(platform, "server.stop", None);
        thread::sleep(Duration::from_millis(300));
    }

    if let Some(pid) = read_pid(&paths::runtime_pid_path(&platform.data_dir)) {
        terminate(pid);
        let _ = fs::remove_file(paths::runtime_pid_path(&platform.data_dir));
    }

    // Best-effort: remove stale socket if process is gone.
    if !client::reachable(platform) && platform.socket.exists() {
        let _ = fs::remove_file(&platform.socket);
    }
    eprintln!("Runtime stopped");
    Ok(())
}

pub fn restart(platform: &Platform, config: &PlatformConfig) -> Result<(), Error> {
    down(platform)?;
    up(platform, config)
}

pub fn status(platform: &Platform, json: bool) -> Result<(), Error> {
    let running = client::reachable(platform);
    let pid = read_pid(&paths::runtime_pid_path(&platform.data_dir));
    let value = json!({
        "mode": platform.mode.as_str(),
        "build": platform.build.as_str(),
        "running": running,
        "pid": pid,
        "socket": platform.socket.display().to_string(),
        "data_dir": platform.data_dir.display().to_string(),
        "bin_dir": platform.bin_dir.display().to_string(),
        "bin": platform.vd_srv().display().to_string(),
    });
    crate::output::emit_value(json, value, |v| {
        println!("Mode        {}", v["mode"].as_str().unwrap_or(""));
        println!("Build       {}", v["build"].as_str().unwrap_or(""));
        println!(
            "Runtime     {}",
            if running { "Running" } else { "Stopped" }
        );
        if let Some(p) = pid {
            println!("PID         {p}");
        }
        println!("Socket      {}", platform.socket.display());
        println!("Data        {}", platform.data_dir.display());
        println!("Bin         {}", platform.bin_dir.display());
    })
}

pub fn wait(platform: &Platform, timeout_secs: u64) -> Result<(), Error> {
    wait_ready(platform, Duration::from_secs(timeout_secs))?;
    eprintln!("Runtime ready");
    Ok(())
}

fn wait_ready(platform: &Platform, timeout: Duration) -> Result<(), Error> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if client::reachable(platform) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(150));
    }
    Err(Error::NotReachable(format!(
        "Runtime not ready within {}s (socket {})",
        timeout.as_secs(),
        platform.socket.display()
    )))
}

fn write_pid(path: &Path, pid: u32) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::Message(e.to_string()))?;
    }
    fs::write(path, pid.to_string()).map_err(|e| Error::Message(e.to_string()))
}

fn read_pid(path: &Path) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn terminate(pid: u32) {
    #[cfg(unix)]
    {
        let alive = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if alive {
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
            thread::sleep(Duration::from_millis(200));
        }
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();
    }
    let _ = pid;
}
