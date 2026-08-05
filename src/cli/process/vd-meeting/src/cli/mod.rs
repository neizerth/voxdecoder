//! Commands for `vd-meeting`.

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
    Plan(RunArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-meeting",
    version,
    about = "Meeting Planner: MeetingRequest → Job → shared Executor",
    long_about = "Plans a meeting Job DAG and submits it to the vd-pipeline Executor.\n\n\
Shorthand: vd-meeting meeting.yaml  ≡  vd-meeting run meeting.yaml"
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Plan Job and submit (or dry-run)
    Run(RunCli),
    /// Plan Job and print it (dry-run)
    Plan(RunCli),
    /// Show or change defaults
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct RunCli {
    /// Meeting document (inputs + meeting model)
    #[arg(value_name = "MEETING")]
    pub meeting_file: Option<PathBuf>,
    #[arg(short = 'f', long = "meeting")]
    pub meeting_flag: Option<PathBuf>,
    /// Repeatable: role=room,path=…[,participant=…][,purposes=transcript|timeline]
    #[arg(long = "input")]
    pub inputs: Vec<String>,
    #[arg(long = "context")]
    pub context: Option<PathBuf>,
    #[arg(short = 'd', long = "output-dir")]
    pub output_dir: Option<PathBuf>,
    #[arg(long = "working-dir")]
    pub working_dir: Option<PathBuf>,
    #[arg(long = "asr")]
    pub asr: Option<String>,
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,
    /// Inference device for ASR (cpu|metal|auto)
    #[arg(long = "device")]
    pub device: Option<String>,
    /// Preprocess playback speed (e.g. 1.5 / 2.0 / 2.2)
    #[arg(long = "speed")]
    pub speed: Option<f64>,
    #[arg(long = "overwrite")]
    pub overwrite: bool,
    #[arg(long = "max-parallel")]
    pub max_parallel: Option<u32>,
    #[arg(long = "continue-on-error")]
    pub continue_on_error: bool,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    #[arg(long = "json")]
    pub json: bool,
    #[arg(long = "progress", value_enum, require_equals = true)]
    pub progress: Option<CliProgress>,
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
    /// Enable interactive mode (auto-detect by TTY if not specified)
    #[arg(long = "interactive")]
    pub interactive: bool,
    /// Disable interactive mode (non-TTY default)
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
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
        RootCommand::Run(cli) => Ok(Command::Run(validate_run(cli, false)?)),
        RootCommand::Plan(cli) => Ok(Command::Plan(validate_run(cli, true)?)),
        RootCommand::Config(c) => Ok(Command::Config(c)),
    }
}

pub fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Run(args) | Command::Plan(args) => run::execute(args),
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
        Some("run" | "plan" | "config" | "help" | "--help" | "-h" | "--version" | "-V")
    );
    if !known {
        args.insert(1, OsString::from("run"));
    }
    args
}

fn validate_run(cli: RunCli, force_dry: bool) -> Result<RunArgs, CliError> {
    if cli.json && !(cli.dry_run || force_dry) {
        return Err(CliError::usage("--json requires --dry-run (or use plan)"));
    }
    if cli.interactive && cli.non_interactive {
        return Err(CliError::usage("cannot use both --interactive and --non-interactive"));
    }
    let meeting_file = cli.meeting_flag.or(cli.meeting_file);
    if meeting_file.is_none() && cli.inputs.is_empty() {
        return Err(CliError::usage("need a meeting document and/or --input …"));
    }
    // Decide interactive mode: explicit flag > auto-detect TTY > default to false
    let interactive = if cli.interactive {
        true
    } else if cli.non_interactive {
        false
    } else {
        atty::is(atty::Stream::Stdin)
    };
    Ok(RunArgs {
        meeting_file,
        inputs: cli.inputs,
        context: cli.context,
        output_dir: cli.output_dir,
        working_dir: cli.working_dir,
        asr: cli.asr,
        model: cli.model,
        device: cli.device,
        speed: cli.speed,
        overwrite: cli.overwrite,
        max_parallel: cli.max_parallel,
        continue_on_error: cli.continue_on_error,
        dry_run: cli.dry_run || force_dry,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
        interactive,
    })
}
