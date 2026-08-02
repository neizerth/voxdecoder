//! Commands for `vd-url`.

mod config_cmd;
mod doctor;
mod inspect;
mod providers;
mod run;
mod validate;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::import::SubtitlePolicy;

#[derive(Debug)]
pub enum Command {
    Run(RunArgs),
    Inspect(InspectArgs),
    Validate(ValidateArgs),
    Providers,
    Doctor,
    Config(ConfigArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-url",
    version,
    about = "Online media importer: URL → ImportResult artifacts",
    long_about = "CLI and Runtime capability share one import library (use: import-url).\n\n\
Shorthand: vd-url -i URL  ≡  vd-url run -i URL"
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Import URL → ImportResult
    Run(RunCli),
    /// Metadata only (no audio download)
    Inspect(InspectCli),
    /// Local checks: URL · provider · subtitle policy (no network)
    Validate(ValidateCli),
    /// List resolvers and capabilities
    Providers,
    /// Check yt-dlp / ffmpeg
    Doctor,
    /// Show or change defaults
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
struct RunCli {
    #[arg(short = 'i', long = "input")]
    input: Option<String>,
    #[arg(short = 'd', long = "output-dir")]
    output_dir: Option<PathBuf>,
    #[arg(long = "subtitles", default_value = "ignore")]
    subtitles: String,
    #[arg(long = "provider", default_value = "auto")]
    provider: String,
    #[arg(long = "metadata-only")]
    metadata_only: bool,
    #[arg(long = "overwrite")]
    overwrite: bool,
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,
}

#[derive(Debug, Clone, Parser)]
struct InspectCli {
    #[arg(short = 'i', long = "input")]
    input: Option<String>,
    #[arg(short = 'd', long = "output-dir")]
    output_dir: Option<PathBuf>,
    #[arg(long = "provider", default_value = "auto")]
    provider: String,
    #[arg(long = "overwrite")]
    overwrite: bool,
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,
}

#[derive(Debug, Clone, Parser)]
struct ValidateCli {
    #[arg(short = 'i', long = "input")]
    input: Option<String>,
    #[arg(long = "subtitles", default_value = "ignore")]
    subtitles: String,
    #[arg(long = "provider", default_value = "auto")]
    provider: String,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: String,
    pub output_dir: PathBuf,
    pub subtitles: SubtitlePolicy,
    pub provider: Option<String>,
    pub metadata_only: bool,
    pub overwrite: bool,
    pub quiet: bool,
    pub output: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct InspectArgs {
    pub input: String,
    pub output_dir: PathBuf,
    pub provider: Option<String>,
    pub overwrite: bool,
    pub quiet: bool,
    pub output: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct ValidateArgs {
    pub input: String,
    pub subtitles: SubtitlePolicy,
    pub provider: Option<String>,
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

    pub fn from_import(err: crate::import::ImportError) -> Self {
        Self {
            code: err.exit_code(),
            message: err.to_string(),
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
        match e.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                CliError::with_code(0, "")
            }
            _ => CliError::usage(""),
        }
    })?;
    match root.command {
        RootCommand::Run(cli) => Ok(Command::Run(validate_run(cli)?)),
        RootCommand::Inspect(cli) => Ok(Command::Inspect(validate_inspect(cli)?)),
        RootCommand::Validate(cli) => Ok(Command::Validate(validate_validate(cli)?)),
        RootCommand::Providers => Ok(Command::Providers),
        RootCommand::Doctor => Ok(Command::Doctor),
        RootCommand::Config(c) => Ok(Command::Config(c)),
    }
}

pub fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Run(args) => run::execute(args),
        Command::Inspect(args) => inspect::execute(args),
        Command::Validate(args) => validate::execute(args),
        Command::Providers => providers::execute(),
        Command::Doctor => doctor::execute(),
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
            "run" | "inspect" | "validate" | "providers" | "doctor" | "config" | "help" | "--help"
                | "-h" | "--version" | "-V"
        )
    );
    if !known {
        args.insert(1, OsString::from("run"));
    }
    args
}

fn provider_hint(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(t.to_string())
    }
}

fn default_output_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn validate_run(cli: RunCli) -> Result<RunArgs, CliError> {
    let input = cli
        .input
        .ok_or_else(|| CliError::usage("no input specified (-i)"))?;
    let subtitles = SubtitlePolicy::parse(&cli.subtitles).map_err(CliError::usage)?;
    Ok(RunArgs {
        input,
        output_dir: cli.output_dir.unwrap_or_else(default_output_dir),
        subtitles,
        provider: provider_hint(&cli.provider),
        metadata_only: cli.metadata_only,
        overwrite: cli.overwrite,
        quiet: cli.quiet,
        output: cli.output,
    })
}

fn validate_inspect(cli: InspectCli) -> Result<InspectArgs, CliError> {
    let input = cli
        .input
        .ok_or_else(|| CliError::usage("no input specified (-i)"))?;
    Ok(InspectArgs {
        input,
        output_dir: cli.output_dir.unwrap_or_else(default_output_dir),
        provider: provider_hint(&cli.provider),
        overwrite: cli.overwrite,
        quiet: cli.quiet,
        output: cli.output,
    })
}

fn validate_validate(cli: ValidateCli) -> Result<ValidateArgs, CliError> {
    let input = cli
        .input
        .ok_or_else(|| CliError::usage("no input specified (-i)"))?;
    let subtitles = SubtitlePolicy::parse(&cli.subtitles).map_err(CliError::usage)?;
    Ok(ValidateArgs {
        input,
        subtitles,
        provider: provider_hint(&cli.provider),
    })
}
