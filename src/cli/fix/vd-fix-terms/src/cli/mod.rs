//! Commands from `cli.md`.

mod config_cmd;
mod run;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

use clap::{builder::PossibleValuesParser, Parser, Subcommand, ValueEnum};

pub use crate::progress::ProgressMode;
use crate::types::Language;

pub use run::RunArgs;

#[derive(Debug)]
pub enum Command {
    Run(RunArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-fix-terms",
    version,
    about = "Local terminology fixer (canonical product / library / protocol names)",
    long_about = "Rewrites only wording needed to lock terms to a canonical form. The input artifact type and structure are preserved.\n\n\
Shorthand: vd-fix-terms -i FILE  ≡  vd-fix-terms run -i FILE\n\n\
Default language: ru (Russian with English insertions)."
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Lock terminology to canonical forms in a local text artifact
    Run(RunCli),
    /// Show or change default settings
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Lock terminology to canonical forms in a local text artifact",
    after_help = "Examples:\n  \
vd-fix-terms run -i meeting.txt\n  \
vd-fix-terms -i meeting.txt --terms ./assets\n  \
vd-fix-terms -i meeting.txt --dry-run"
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
    /// Project assets from `vd-assets` (default: nearest `.voxdecoder` if present). Repeatable; last wins.
    #[arg(long = "terms")]
    pub terms: Vec<PathBuf>,
    /// Disable the shipping lexicon (corporate-only glossaries).
    #[arg(long = "no-shipping-lexicon")]
    pub no_shipping_lexicon: bool,
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
    Ok(RunArgs {
        input: cli.input,
        output: cli.output,
        output_dir: cli.output_dir,
        in_place: cli.in_place,
        overwrite: cli.overwrite,
        language,
        terms: cli.terms,
        shipping: !cli.no_shipping_lexicon,
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
    })
}
