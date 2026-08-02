//! Command-line interface for the MCP gateway.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Instant;

use clap::{Parser, Subcommand};
use serde_json::json;

use crate::client::{resolve, RuntimeClient};
use crate::config::{self, GatewayConfig};
use crate::error::Error;
use crate::mcp;
use crate::paths;

#[derive(Debug, Parser)]
#[command(name = "vd-mcp", version, about = "VoxDecoder MCP gateway")]
struct Root {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    transport: Option<String>,
    #[arg(long, global = true)]
    tcp: Option<String>,
    #[arg(long, global = true)]
    socket: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve,
    Ping,
    Info,
    Doctor,
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    Get { key: String },
    Set { key: String, value: String },
    Path,
    List,
}

pub fn run<I, T>(args: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let root = Root::try_parse_from(args).map_err(|e| Error::Message(e.to_string()))?;
    let config_path = root.config.unwrap_or_else(paths::config_path);
    match root.command.unwrap_or(Command::Serve) {
        Command::Config { action } => config_command(&config_path, action),
        command => {
            let config = config::load(&config_path).map_err(Error::from)?;
            let client = RuntimeClient::new(
                resolve(
                    &config,
                    root.transport.as_deref(),
                    root.tcp.as_deref(),
                    root.socket.as_deref(),
                )
                .map_err(Error::from)?,
            );
            match command {
                Command::Serve => mcp::serve(client).map_err(Error::from),
                Command::Ping => {
                    println!("{}", client.call("server.ping", None).map_err(Error::from)?);
                    Ok(())
                }
                Command::Info => info(&client),
                Command::Doctor => doctor(&client),
                Command::Config { .. } => unreachable!(),
            }
        }
    }
}

fn info(client: &RuntimeClient) -> Result<(), Error> {
    let started = Instant::now();
    let version = client.call("server.version", None).map_err(Error::from)?;
    let info = client.call("server.info", None).map_err(Error::from)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "endpoint": client.endpoint().display(),
            "latency_ms": started.elapsed().as_millis(),
            "version": version,
            "server_info": info,
        }))
        .map_err(|e| Error::Message(e.to_string()))?
    );
    Ok(())
}

fn doctor(client: &RuntimeClient) -> Result<(), Error> {
    let started = Instant::now();
    let ping = client.call("server.ping", None).map_err(Error::from)?;
    let info = client.call("server.info", None).map_err(Error::from)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": "ok",
            "transport_endpoint": client.endpoint().display(),
            "latency_ms": started.elapsed().as_millis(),
            "ping": ping,
            "api_version": info.get("api_version"),
            "auth": "n/a",
        }))
        .map_err(|e| Error::Message(e.to_string()))?
    );
    Ok(())
}

fn config_command(path: &std::path::Path, action: ConfigAction) -> Result<(), Error> {
    match action {
        ConfigAction::Path => println!("{}", path.display()),
        ConfigAction::List => {
            let cfg = config::load(path).map_err(Error::from)?;
            println!(
                "{}",
                toml::to_string_pretty(&cfg).map_err(|e| Error::Message(e.to_string()))?
            );
        }
        ConfigAction::Get { key } => {
            let cfg = config::load(path).map_err(Error::from)?;
            println!("{}", get(&cfg, &key)?);
        }
        ConfigAction::Set { key, value } => {
            let mut cfg = config::load(path).map_err(Error::from)?;
            set(&mut cfg, &key, value)?;
            config::save(path, &cfg).map_err(Error::from)?;
        }
    }
    Ok(())
}

fn get(config: &GatewayConfig, key: &str) -> Result<String, Error> {
    match key {
        "transport" => Ok(config.transport.clone().unwrap_or_default()),
        "tcp" => Ok(config.tcp.clone().unwrap_or_default()),
        "socket" => Ok(config
            .socket
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()),
        "data_dir" => Ok(config
            .data_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()),
        _ => Err(Error::Message(format!("unknown key: {key}"))),
    }
}

fn set(config: &mut GatewayConfig, key: &str, value: String) -> Result<(), Error> {
    match key {
        "transport" => config.transport = Some(value),
        "tcp" => config.tcp = Some(value),
        "socket" => config.socket = Some(PathBuf::from(value)),
        "data_dir" => config.data_dir = Some(PathBuf::from(value)),
        _ => return Err(Error::Message(format!("unknown key: {key}"))),
    }
    Ok(())
}
