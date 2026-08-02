//! CLI for `vd-srv`.

mod serve;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};

use crate::api::{self, Request, Response};
use crate::config;
use crate::paths;

#[derive(Debug)]
pub enum Command {
    Serve(ServeArgs),
    Stop,
    Ping,
    Health,
    Submit(SubmitArgs),
    Queue,
    Jobs,
    JobInfo { id: String },
    Watch { id: String },
    Events { id: String, follow: bool },
    Logs { id: String, follow: bool, stderr: bool },
    Artifacts { id: String },
    Cancel { id: String },
    Top,
    Config(ConfigArgs),
}

#[derive(Debug, Clone)]
pub struct ServeArgs {
    pub data_dir: Option<PathBuf>,
    pub socket: Option<PathBuf>,
    pub workers: Option<u32>,
    pub foreground: bool,
}

#[derive(Debug, Clone)]
pub struct SubmitArgs {
    pub job: Option<PathBuf>,
    pub stdin: bool,
    pub priority: Option<String>,
    pub restart: Option<String>,
    pub wait: bool,
    pub json: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigArgs {
    pub action: ConfigAction,
}

#[derive(Debug, Clone)]
pub enum ConfigAction {
    List,
    Get { key: String },
    Set { key: String, value: String },
    Path,
}

#[derive(Debug)]
pub struct CliError {
    code: u8,
    message: String,
}

impl CliError {
    pub fn usage(msg: impl Into<String>) -> Self {
        Self {
            code: 2,
            message: msg.into(),
        }
    }
    pub fn with_code(code: u8, msg: impl Into<String>) -> Self {
        Self {
            code,
            message: msg.into(),
        }
    }
    pub fn exit_code(&self) -> u8 {
        self.code
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-srv",
    version,
    about = "VoxDecoder execution engine",
    long_about = "Schedule Jobs, persist state, observe progress.\n\n\
Capability work runs through the shared vd-pipeline Executor."
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Start the local server
    Serve {
        #[arg(long = "data-dir")]
        data_dir: Option<PathBuf>,
        #[arg(long = "socket")]
        socket: Option<PathBuf>,
        #[arg(long = "workers")]
        workers: Option<u32>,
        #[arg(long = "foreground", default_value_t = true)]
        foreground: bool,
    },
    Stop,
    Ping,
    Health,
    /// Enqueue a Job (file or `-` for stdin)
    Submit {
        #[arg(value_name = "JOB")]
        job: Option<PathBuf>,
        #[arg(long = "priority")]
        priority: Option<String>,
        #[arg(long = "restart")]
        restart: Option<String>,
        #[arg(long = "wait")]
        wait: bool,
        #[arg(long = "json")]
        json: bool,
    },
    Queue,
    Jobs,
    #[command(name = "job")]
    Job {
        #[command(subcommand)]
        action: JobAction,
    },
    Watch {
        id: String,
    },
    Events {
        id: String,
        #[arg(long = "follow")]
        follow: bool,
    },
    Logs {
        id: String,
        #[arg(long = "follow")]
        follow: bool,
        #[arg(long = "stderr")]
        stderr: bool,
    },
    Artifacts {
        id: String,
    },
    Cancel {
        id: String,
    },
    Top,
    Config {
        #[command(subcommand)]
        action: ConfigCli,
    },
}

#[derive(Debug, Subcommand)]
enum JobAction {
    Info { id: String },
}

#[derive(Debug, Subcommand)]
enum ConfigCli {
    List,
    Get { key: String },
    Set { key: String, value: String },
    Path,
}

pub fn parse_args<I, T>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let root = Root::try_parse_from(args).map_err(|e| {
        let msg = e.to_string();
        if matches!(
            e.kind(),
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
        ) {
            CliError {
                code: 0,
                message: msg,
            }
        } else {
            CliError::usage(msg)
        }
    })?;
    Ok(match root.command {
        RootCommand::Serve {
            data_dir,
            socket,
            workers,
            foreground,
        } => Command::Serve(ServeArgs {
            data_dir,
            socket,
            workers,
            foreground,
        }),
        RootCommand::Stop => Command::Stop,
        RootCommand::Ping => Command::Ping,
        RootCommand::Health => Command::Health,
        RootCommand::Submit {
            job,
            priority,
            restart,
            wait,
            json,
        } => {
            let stdin = job.as_ref().is_some_and(|p| p.as_os_str() == OsStr::new("-"));
            Command::Submit(SubmitArgs {
                job: if stdin { None } else { job },
                stdin,
                priority,
                restart,
                wait,
                json,
            })
        }
        RootCommand::Queue => Command::Queue,
        RootCommand::Jobs => Command::Jobs,
        RootCommand::Job {
            action: JobAction::Info { id },
        } => Command::JobInfo { id },
        RootCommand::Watch { id } => Command::Watch { id },
        RootCommand::Events { id, follow } => Command::Events { id, follow },
        RootCommand::Logs { id, follow, stderr } => Command::Logs { id, follow, stderr },
        RootCommand::Artifacts { id } => Command::Artifacts { id },
        RootCommand::Cancel { id } => Command::Cancel { id },
        RootCommand::Top => Command::Top,
        RootCommand::Config { action } => Command::Config(ConfigArgs {
            action: match action {
                ConfigCli::List => ConfigAction::List,
                ConfigCli::Get { key } => ConfigAction::Get { key },
                ConfigCli::Set { key, value } => ConfigAction::Set { key, value },
                ConfigCli::Path => ConfigAction::Path,
            },
        }),
    })
}

pub fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Serve(args) => serve::run(args),
        Command::Config(args) => config_cmd(args),
        other => client_cmd(other),
    }
}

fn socket_path() -> Result<PathBuf, CliError> {
    let cfg = config::load(&paths::config_path()).map_err(CliError::usage)?;
    let data = config::effective_data_dir(&cfg.raw, None);
    Ok(config::effective_socket(&cfg.raw, &data))
}

fn client_cmd(cmd: Command) -> Result<(), CliError> {
    let sock = socket_path()?;
    match cmd {
        Command::Ping => {
            let resp = call(&sock, &Request::Ping)?;
            print_resp(&resp, false)?;
            Ok(())
        }
        Command::Stop => {
            let resp = call(&sock, &Request::Stop)?;
            print_resp(&resp, false)?;
            Ok(())
        }
        Command::Health | Command::Top => {
            let resp = call(&sock, &Request::Health)?;
            print_resp(&resp, true)?;
            Ok(())
        }
        Command::Queue | Command::Jobs => {
            let resp = call(&sock, &Request::Queue)?;
            print_resp(&resp, true)?;
            Ok(())
        }
        Command::Submit(args) => submit_cmd(&sock, args),
        Command::JobInfo { id } => {
            let resp = call(&sock, &Request::JobInfo { id })?;
            print_resp(&resp, true)?;
            Ok(())
        }
        Command::Events { id, follow } => follow_events(&sock, &id, follow),
        Command::Watch { id } => follow_events(&sock, &id, true),
        Command::Logs {
            id,
            follow,
            stderr,
        } => logs_cmd(&sock, &id, follow, stderr),
        Command::Artifacts { id } => {
            let resp = call(&sock, &Request::Artifacts { id })?;
            if let Some(data) = &resp.data {
                if let Some(arr) = data.as_array() {
                    for a in arr {
                        if let Some(p) = a.get("path").and_then(|v| v.as_str()) {
                            println!("{p}");
                        }
                    }
                } else {
                    print_resp(&resp, true)?;
                }
            } else {
                print_resp(&resp, false)?;
            }
            Ok(())
        }
        Command::Cancel { id } => {
            let resp = call(&sock, &Request::Cancel { id })?;
            print_resp(&resp, true)?;
            Ok(())
        }
        Command::Serve(_) | Command::Config(_) => unreachable!(),
    }
}

fn submit_cmd(sock: &Path, args: SubmitArgs) -> Result<(), CliError> {
    let raw = if args.stdin {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| CliError::with_code(1, e.to_string()))?;
        buf
    } else {
        let path = args
            .job
            .ok_or_else(|| CliError::usage("submit needs JOB path or -"))?;
        fs::read_to_string(&path).map_err(|e| CliError::with_code(3, e.to_string()))?
    };
    let resp = call(
        sock,
        &Request::Submit {
            job: None,
            job_yaml: Some(raw),
            priority: args.priority.clone(),
            restart: args.restart.clone(),
        },
    )?;
    if !resp.ok {
        return Err(CliError::with_code(
            1,
            resp.error.unwrap_or_else(|| "submit failed".into()),
        ));
    }
    let id = resp
        .data
        .as_ref()
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&resp.data).unwrap_or_default());
    } else {
        println!("job: {id}");
    }
    if args.wait && !id.is_empty() {
        wait_job(sock, &id, args.json)?;
    }
    Ok(())
}

fn wait_job(sock: &Path, id: &str, json: bool) -> Result<(), CliError> {
    loop {
        let resp = call(sock, &Request::JobInfo { id: id.into() })?;
        let status = resp
            .data
            .as_ref()
            .and_then(|d| d.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if matches!(status, "completed" | "failed" | "cancelled") {
            if json {
                println!("{}", serde_json::to_string_pretty(&resp.data).unwrap_or_default());
            } else {
                println!("status: {status}");
            }
            if status == "failed" {
                return Err(CliError::with_code(1, "job failed".to_string()));
            }
            if status == "cancelled" {
                return Err(CliError::with_code(4, "job cancelled".to_string()));
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(300));
    }
}

fn follow_events(sock: &Path, id: &str, follow: bool) -> Result<(), CliError> {
    let mut seen = 0usize;
    loop {
        let resp = call(sock, &Request::Events { id: id.into() })?;
        ensure_ok(&resp)?;
        if let Some(arr) = resp.data.as_ref().and_then(|d| d.as_array()) {
            for ev in arr.iter().skip(seen) {
                let kind = ev.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
                let msg = ev
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if msg.is_empty() {
                    println!("{kind}");
                } else {
                    println!("{kind}: {msg}");
                }
            }
            seen = arr.len();
            let done = arr.iter().any(|e| {
                matches!(
                    e.get("kind").and_then(|v| v.as_str()),
                    Some("JobFinished" | "JobFailed" | "JobCancelled")
                )
            });
            if done || !follow {
                return Ok(());
            }
        } else if !follow {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(300));
    }
}

fn logs_cmd(sock: &Path, id: &str, follow: bool, stderr: bool) -> Result<(), CliError> {
    let mut last_len = 0usize;
    loop {
        let resp = call(
            sock,
            &Request::Logs {
                id: id.into(),
                stderr,
            },
        )?;
        ensure_ok(&resp)?;
        let text = resp
            .data
            .as_ref()
            .and_then(|d| d.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if text.len() > last_len {
            print!("{}", &text[last_len..]);
            last_len = text.len();
        }
        if !follow {
            return Ok(());
        }
        let info = call(sock, &Request::JobInfo { id: id.into() })?;
        let status = info
            .data
            .as_ref()
            .and_then(|d| d.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if matches!(status, "completed" | "failed" | "cancelled") {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(300));
    }
}

fn call(sock: &Path, req: &Request) -> Result<Response, CliError> {
    api::call(sock, req).map_err(|e| CliError::with_code(3, e))
}

fn ensure_ok(resp: &Response) -> Result<(), CliError> {
    if resp.ok {
        Ok(())
    } else {
        Err(CliError::with_code(
            1,
            resp.error.clone().unwrap_or_else(|| "error".into()),
        ))
    }
}

fn print_resp(resp: &Response, pretty: bool) -> Result<(), CliError> {
    ensure_ok(resp)?;
    if let Some(data) = &resp.data {
        if pretty {
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
        } else {
            println!("{data}");
        }
    }
    Ok(())
}

fn config_cmd(args: ConfigArgs) -> Result<(), CliError> {
    let path = paths::config_path();
    match args.action {
        ConfigAction::Path => {
            println!("{}", path.display());
            Ok(())
        }
        ConfigAction::List => {
            let cfg = config::load(&path).map_err(CliError::usage)?;
            println!(
                "{}",
                toml::to_string_pretty(&cfg.raw).map_err(|e| CliError::with_code(1, e.to_string()))?
            );
            Ok(())
        }
        ConfigAction::Get { key } => {
            let cfg = config::load(&path).map_err(CliError::usage)?;
            let v = match key.as_str() {
                "workers" => cfg.raw.workers.to_string(),
                "log_level" => cfg.raw.log_level.clone(),
                "history" => cfg.raw.history.to_string(),
                "data_dir" => cfg
                    .raw
                    .data_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                "socket" => cfg
                    .raw
                    .socket
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                other => {
                    return Err(CliError::usage(format!("unknown key: {other}")));
                }
            };
            println!("{v}");
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut cfg = config::load(&path).map_err(CliError::usage)?;
            match key.as_str() {
                "workers" => {
                    cfg.raw.workers = value
                        .parse()
                        .map_err(|_| CliError::usage("workers must be u32"))?;
                }
                "log_level" => cfg.raw.log_level = value,
                "history" => {
                    cfg.raw.history = value
                        .parse()
                        .map_err(|_| CliError::usage("history must be u32"))?;
                }
                "data_dir" => cfg.raw.data_dir = Some(PathBuf::from(value)),
                "socket" => cfg.raw.socket = Some(PathBuf::from(value)),
                other => return Err(CliError::usage(format!("unknown key: {other}"))),
            }
            config::save(&path, &cfg.raw).map_err(|e| CliError::with_code(1, e))?;
            Ok(())
        }
    }
}
