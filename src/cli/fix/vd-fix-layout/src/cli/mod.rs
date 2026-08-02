//! Commands from `cli.md`.

mod config_cmd;
mod info;
mod install;
mod list;
mod remove;
mod run;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

use clap::{builder::PossibleValuesParser, Parser, Subcommand, ValueEnum};

pub use crate::progress::ProgressMode;
use crate::config::parse_layout_language;
use crate::types::ParagraphDensity;

pub use run::RunArgs;

#[derive(Debug)]
pub enum Command {
    Run(RunArgs),
    Config(ConfigArgs),
    Install(InstallArgs),
    Remove(RemoveArgs),
    List(ListArgs),
    Info(InfoArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-fix-layout",
    version,
    about = "Local layout fixer (paragraph / block boundaries)",
    long_about = "Never changes lexical content. Only whitespace and paragraph / block boundaries may change.\n\n\
Shorthand: vd-fix-layout -i FILE  ≡  vd-fix-layout run -i FILE\n\n\
Language packs are optional for the built-in rules backend (vd-fix-layout install ru)."
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Apply layout to a local text artifact
    Run(RunCli),
    /// Download / install a language pack
    Install(InstallArgs),
    /// Remove an installed language pack
    Remove(RemoveArgs),
    /// List language packs
    List(ListArgs),
    /// Show pack metadata
    Info(InfoArgs),
    /// Show or change default settings
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Apply layout to a local text artifact",
    after_help = "Examples:\n  \
vd-fix-layout install ru\n  \
vd-fix-layout run -i meeting.txt --language auto\n  \
vd-fix-layout -i meeting.txt --dry-run"
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
        value_parser = PossibleValuesParser::new(["ru", "en", "auto"])
    )]
    pub language: Option<String>,
    #[arg(
        long = "density",
        value_parser = PossibleValuesParser::new(ParagraphDensity::allowed())
    )]
    pub density: Option<String>,
    #[arg(long = "timemap")]
    pub timemap: Option<PathBuf>,
    #[arg(long = "no-timemap")]
    pub no_timemap: bool,
    #[arg(long = "download-root")]
    pub download_root: Option<PathBuf>,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    #[arg(long = "json")]
    pub json: bool,
    #[arg(long = "progress", value_enum, require_equals = true)]
    pub progress: Option<CliProgress>,
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Download / install a language pack",
    after_help = "Examples:\n  vd-fix-layout install ru\n  vd-fix-layout install --all"
)]
pub struct InstallArgs {
    pub model: Option<String>,
    #[arg(long = "all")]
    pub all: bool,
    #[arg(long = "download-root")]
    pub download_root: Option<PathBuf>,
    #[arg(long = "force")]
    pub force: bool,
    #[arg(long = "progress", value_enum, require_equals = true)]
    pub progress: Option<CliProgress>,
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct RemoveArgs {
    pub model: String,
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
    #[arg(long = "download-root")]
    pub download_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Parser)]
pub struct ListArgs {
    #[arg(long = "all")]
    pub all: bool,
    #[arg(long = "format", value_enum, default_value_t = CliListFormat::Text)]
    pub format: CliListFormat,
    #[arg(long = "download-root")]
    pub download_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Parser)]
pub struct InfoArgs {
    pub model: String,
    #[arg(long = "json")]
    pub json: bool,
    #[arg(long = "download-root")]
    pub download_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum CliListFormat {
    #[default]
    Text,
    Json,
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
        RootCommand::Install(a) => {
            if a.model.is_none() && !a.all {
                return Err(CliError::usage(format!(
                    "install requires MODEL or --all\n\n{}",
                    crate::models::catalog_help_lines()
                )));
            }
            if let Some(ref m) = a.model {
                if !a.all && !crate::models::is_catalog_name(m) {
                    return Err(CliError::usage(format!(
                        "unknown model '{m}'\n\n{}",
                        crate::models::catalog_help_lines()
                    )));
                }
            }
            Ok(Command::Install(a))
        }
        RootCommand::Remove(a) => Ok(Command::Remove(a)),
        RootCommand::List(a) => Ok(Command::List(a)),
        RootCommand::Info(a) => Ok(Command::Info(a)),
    }
}

pub fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Run(args) => run::execute(args),
        Command::Config(args) => config_cmd::execute(args),
        Command::Install(args) => install::execute(args),
        Command::Remove(args) => remove::execute(args),
        Command::List(args) => list::execute(args),
        Command::Info(args) => info::execute(args),
    }
}

fn normalize_argv(args: Vec<OsString>) -> Vec<OsString> {
    if args.len() < 2 {
        return args;
    }
    if is_subcommand(&args[1]) {
        return args;
    }
    let mut out = Vec::with_capacity(args.len() + 1);
    out.push(args[0].clone());
    out.push(OsString::from("run"));
    out.extend(args.into_iter().skip(1));
    out
}

fn is_subcommand(s: &OsStr) -> bool {
    matches!(
        s.to_string_lossy().as_ref(),
        "run"
            | "config"
            | "install"
            | "remove"
            | "list"
            | "info"
            | "help"
            | "-h"
            | "--help"
            | "-V"
            | "--version"
    )
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
    if cli.timemap.is_some() && cli.no_timemap {
        return Err(CliError::usage("--timemap and --no-timemap are mutually exclusive"));
    }
    let language = cli
        .language
        .as_deref()
        .map(|s| parse_layout_language(s).map_err(CliError::usage))
        .transpose()?;
    let density = cli
        .density
        .as_deref()
        .map(|s| {
            ParagraphDensity::parse(s)
                .ok_or_else(|| CliError::usage(format!("invalid density: {s}")))
        })
        .transpose()?;

    Ok(RunArgs {
        input: cli.input,
        output: cli.output,
        output_dir: cli.output_dir,
        in_place: cli.in_place,
        overwrite: cli.overwrite,
        language,
        density,
        timemap: cli.timemap,
        no_timemap: cli.no_timemap,
        download_root: cli.download_root,
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
    })
}
