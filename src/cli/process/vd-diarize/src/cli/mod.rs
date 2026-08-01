//! Commands for `vd-diarize`.

mod assets_cmd;
mod config_cmd;
mod run;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

pub use crate::progress::ProgressMode;
pub use run::RunArgs;

#[derive(Debug)]
pub enum Command {
    Run(RunArgs),
    Install { provider: String },
    Remove { provider: String },
    List,
    Info { provider: String },
    Config(ConfigArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-diarize",
    version,
    about = "Local-first speaker diarization (SpeakerTimeline)",
    long_about = "CLI ≡ use: diarize for the shared Executor.\n\n\
Shorthand: vd-diarize -i meeting.wav  ≡  vd-diarize run -i meeting.wav"
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Diarize one audio file → SpeakerTimeline
    Run(RunCli),
    /// Install backend assets
    Install { provider: String },
    /// Remove installed backend assets
    Remove { provider: String },
    /// List installed providers
    List,
    /// Show asset pack info
    Info { provider: String },
    /// Show or change defaults
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct RunCli {
    #[arg(short = 'i', long = "input")]
    pub input: PathBuf,
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    #[arg(short = 'd', long = "output-dir")]
    pub output_dir: Option<PathBuf>,
    #[arg(long = "backend")]
    pub backend: Option<String>,
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,
    #[arg(long = "device")]
    pub device: Option<String>,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    #[arg(long = "json")]
    pub json: bool,
    #[arg(long = "progress", value_enum, require_equals = true)]
    pub progress: Option<CliProgress>,
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
    #[arg(long = "overwrite")]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum CliProgress {
    Text,
    Json,
}

impl From<CliProgress> for ProgressMode {
    fn from(v: CliProgress) -> Self {
        match v {
            CliProgress::Text => Self::Text,
            CliProgress::Json => Self::Json,
        }
    }
}

#[derive(Debug, Clone, Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Debug, Clone, Subcommand)]
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
    pub fn usage(msg: impl AsRef<str>) -> Self {
        Self {
            code: 2,
            message: msg.as_ref().to_string(),
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

impl std::error::Error for CliError {}

pub fn parse_args<I, T>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let args = normalize_argv(args);
    let root = Root::try_parse_from(args).map_err(|e| {
        let _ = e.print();
        CliError::usage("")
    })?;
    match root.command {
        RootCommand::Run(cli) => Ok(Command::Run(validate_run(cli)?)),
        RootCommand::Install { provider } => Ok(Command::Install { provider }),
        RootCommand::Remove { provider } => Ok(Command::Remove { provider }),
        RootCommand::List => Ok(Command::List),
        RootCommand::Info { provider } => Ok(Command::Info { provider }),
        RootCommand::Config(c) => Ok(Command::Config(c)),
    }
}

pub fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Run(args) => run::execute(args),
        Command::Install { provider } => assets_cmd::install(&provider),
        Command::Remove { provider } => assets_cmd::remove(&provider),
        Command::List => assets_cmd::list(),
        Command::Info { provider } => assets_cmd::info(&provider),
        Command::Config(args) => config_cmd::execute(args),
    }
}

fn normalize_argv(mut args: Vec<OsString>) -> Vec<OsString> {
    if args.len() < 2 {
        return args;
    }
    let first = args.get(1).map(OsString::as_os_str);
    let known = matches!(
        first.and_then(OsStr::to_str),
        Some(
            "run" | "install" | "remove" | "list" | "info" | "config" | "help" | "--help"
                | "-h" | "--version" | "-V"
        )
    );
    if !known {
        args.insert(1, OsString::from("run"));
    }
    args
}

fn validate_run(cli: RunCli) -> Result<RunArgs, CliError> {
    if cli.json && !cli.dry_run {
        return Err(CliError::usage("--json requires --dry-run"));
    }
    let mut output = cli.output;
    if output.is_none() {
        if let Some(dir) = &cli.output_dir {
            let stem = cli
                .input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("out");
            output = Some(dir.join(format!("{stem}.diarization.json")));
        }
    }
    Ok(RunArgs {
        input: cli.input,
        output,
        backend: cli.backend,
        model: cli.model,
        device: cli.device,
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
        overwrite: cli.overwrite,
    })
}
