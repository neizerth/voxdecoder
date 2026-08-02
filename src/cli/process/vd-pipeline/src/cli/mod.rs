//! Commands for `vd-pipeline`.

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
    Config(ConfigArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-pipeline",
    version,
    about = "Execute a VoxDecoder Job",
    long_about = "Build or load a Job and run it through the Executor.\n\n\
Shorthand: vd-pipeline -i FILE  ≡  vd-pipeline run -i FILE"
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Build or load a Job, then execute
    Run(RunCli),
    /// Show or change default settings
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Build or load a Job, then execute",
    after_help = "Examples:\n  \
vd-pipeline run -i meeting.ogg\n  \
vd-pipeline run -i meeting.ogg --docs ./docs --dry-run --json\n  \
vd-pipeline run job.yaml"
)]
pub struct RunCli {
    /// Audio/video for default Job (mutually exclusive with job file)
    #[arg(short = 'i', long = "input")]
    pub input: Option<PathBuf>,
    /// Job file (.yaml / .yml / .json)
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,
    /// Positional job file (alternative to --file)
    #[arg(value_name = "JOB")]
    pub job_positional: Option<PathBuf>,
    /// Transcribe engine: gigaam | whisper
    #[arg(long = "asr", default_value = "gigaam")]
    pub asr: String,
    /// Catalog / checkpoint model for transcribe
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,
    /// Inference device for transcribe (e.g. cpu|metal|cuda|auto)
    #[arg(long = "device")]
    pub device: Option<String>,
    /// Request FlashAttention (CUDA builds of vd-gigaam)
    #[arg(long = "flash")]
    pub flash: bool,
    /// Docs root → prepare-context
    #[arg(long = "docs")]
    pub docs: Option<PathBuf>,
    #[arg(short = 'd', long = "output-dir")]
    pub output_dir: Option<PathBuf>,
    #[arg(long = "working-dir")]
    pub working_dir: Option<PathBuf>,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    #[arg(long = "json")]
    pub json: bool,
    #[arg(long = "progress", value_enum, require_equals = true)]
    pub progress: Option<CliProgress>,
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
    #[arg(long = "continue-on-error")]
    pub continue_on_error: bool,
    #[arg(long = "overwrite")]
    pub overwrite: bool,
    /// Write ExecutionReport JSON to this file
    #[arg(long = "report")]
    pub report: Option<PathBuf>,
    /// Write report.json + resolved-job.json into this directory
    #[arg(long = "report-dir")]
    pub report_dir: Option<PathBuf>,
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
        RootCommand::Config(c) => Ok(Command::Config(c)),
    }
}

pub fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Run(args) => run::execute(args),
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
        Some("run" | "config" | "help" | "--help" | "-h" | "--version" | "-V")
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
    if cli.report.is_some() && cli.report_dir.is_some() {
        return Err(CliError::usage(
            "--report and --report-dir are mutually exclusive",
        ));
    }
    if cli.dry_run && (cli.report.is_some() || cli.report_dir.is_some()) {
        return Err(CliError::usage(
            "--report / --report-dir require a real run (not --dry-run)",
        ));
    }
    let job_file = cli.file.or(cli.job_positional);
    if job_file.is_some() && cli.input.is_some() {
        return Err(CliError::usage(
            "job file and -i / --input are mutually exclusive",
        ));
    }
    if job_file.is_none() && cli.input.is_none() {
        return Err(CliError::with_code(
            3,
            "missing -i / --input or job file".to_string(),
        ));
    }
    Ok(RunArgs {
        input: cli.input,
        job_file,
        asr: cli.asr,
        model: cli.model,
        device: cli.device,
        flash: cli.flash,
        docs: cli.docs,
        output_dir: cli.output_dir,
        working_dir: cli.working_dir,
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
        continue_on_error: cli.continue_on_error,
        overwrite: cli.overwrite,
        report: cli.report,
        report_dir: cli.report_dir,
    })
}
