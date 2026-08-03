//! Commands from `cli.md`.

mod config_cmd;
mod run;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

use clap::{builder::PossibleValuesParser, Parser, Subcommand, ValueEnum};

pub use crate::progress::ProgressMode;
use crate::types::{Language, Mode};

pub use run::RunArgs;

#[derive(Debug)]
pub enum Command {
    Run(RunArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-fix-disfluency",
    version,
    about = "Local speech-disfluency cleanup (fillers, false starts, empty hesitations)",
    long_about = "Removes speech noise. Never removes information. The input artifact type and structure are preserved.\n\n\
Shorthand: vd-fix-disfluency -i FILE  ≡  vd-fix-disfluency run -i FILE\n\n\
Default language: ru. Default mode: light."
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Remove speech disfluencies from a local text artifact
    Run(RunCli),
    /// Show or change default settings
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Remove speech disfluencies from a local text artifact",
    after_help = "Examples:\n  \
vd-fix-disfluency run -i meeting.txt\n  \
vd-fix-disfluency -i meeting.txt --mode normal\n  \
vd-fix-disfluency -i meeting.txt --dry-run"
)]
pub struct RunCli {
    #[arg(short = 'i', long = "input", required = true)]
    pub input: PathBuf,
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    #[arg(short = 'd', long = "output-dir")]
    pub output_dir: Option<PathBuf>,
    #[arg(long = "in-place")]
    pub in_place: bool,
    #[arg(long = "overwrite")]
    pub overwrite: bool,
    #[arg(
        short = 'l',
        long = "language",
        value_parser = PossibleValuesParser::new(Language::allowed())
    )]
    pub language: Option<String>,
    #[arg(
        short = 'm',
        long = "mode",
        value_parser = PossibleValuesParser::new(Mode::allowed())
    )]
    pub mode: Option<String>,
    #[arg(long = "no-fillers")]
    pub no_fillers: bool,
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
    let targets = u8::from(cli.output.is_some())
        + u8::from(cli.output_dir.is_some())
        + u8::from(cli.in_place);
    if targets > 1 {
        return Err(CliError::usage(
            "--output, --output-dir, and --in-place are mutually exclusive",
        ));
    }
    if cli.json && !cli.dry_run {
        return Err(CliError::usage("--json requires --dry-run"));
    }
    let language = cli
        .language
        .as_deref()
        .map(|s| {
            Language::parse(s).ok_or_else(|| CliError::usage(format!("invalid language: {s}")))
        })
        .transpose()?;
    let mode = cli
        .mode
        .as_deref()
        .map(|s| Mode::parse(s).ok_or_else(|| CliError::usage(format!("invalid mode: {s}"))))
        .transpose()?;
    Ok(RunArgs {
        input: cli.input,
        output: cli.output,
        output_dir: cli.output_dir,
        in_place: cli.in_place,
        overwrite: cli.overwrite,
        language,
        mode,
        remove_fillers: if cli.no_fillers { Some(false) } else { None },
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
    })
}
