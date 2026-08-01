//! Commands for `vd-assets`.

mod config_cmd;
mod run;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

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
    name = "vd-assets",
    version,
    about = "Build reusable project assets for vd-fix-*",
    long_about = "Prepare project knowledge for vd-fix-asr / vd-fix-terms: Markdown from docs (optional OCR) and a terms.yml bundle.\n\n\
Shorthand: vd-assets -i ./docs -o ./assets  ≡  vd-assets run -i ./docs -o ./assets"
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Build project assets (Markdown + terms.yml)
    Run(RunCli),
    /// Show or change default settings
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Build project assets directory for vd-fix-*",
    after_help = "Examples:\n  \
vd-assets run -i ./docs -o ./assets\n  \
vd-assets -i ./spec.pdf -o ./out --ocr\n  \
vd-fix-terms run -i meeting.txt --terms ./assets\n  \
vd-fix-asr run -i meeting.txt --context ./assets"
)]
pub struct RunCli {
    /// Input file or directory (repeatable)
    #[arg(short = 'i', long = "input", required = true)]
    pub input: Vec<PathBuf>,
    /// Output assets directory (`md/` + `terms.yml`). Default: `.voxdecoder` (or `$VD_PROJECT_DIR`)
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    /// Enable OCR for scanned documents
    #[arg(long = "ocr")]
    pub ocr: bool,
    /// Rebuild even if extract cache matches
    #[arg(long = "force")]
    pub force: bool,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    #[arg(long = "json")]
    pub json: bool,
    #[arg(long = "progress", value_enum, require_equals = true)]
    pub progress: Option<CliProgress>,
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
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

    pub fn with_code(code: u8, msg: impl AsRef<str>) -> Self {
        Self {
            code,
            message: msg.as_ref().to_string(),
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
    if cli.input.is_empty() {
        return Err(CliError::usage("at least one --input is required"));
    }
    Ok(RunArgs {
        input: cli.input.clone(),
        output: cli.output.unwrap_or_else(|| {
            let start = cli.input.first().map(PathBuf::as_path).unwrap_or_else(|| Path::new("."));
            vd_artifact::paths::project_dir(start)
        }),
        ocr: cli.ocr,
        force: cli.force,
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
    })
}
