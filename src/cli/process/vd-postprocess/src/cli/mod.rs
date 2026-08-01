//! Commands for `vd-postprocess`.

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
    name = "vd-postprocess",
    version,
    about = "Recipe executor: artifacts + recipes + provider → derived artifacts",
    long_about = "CLI ≡ use: postprocess. No recipes → error.\n\n\
Shorthand: vd-postprocess --input a=f.txt --recipe r.yaml  ≡  vd-postprocess run …"
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Apply recipes to named inputs
    Run(RunCli),
    /// Show or change defaults
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct RunCli {
    /// Named input: name=path (repeatable)
    #[arg(long = "input")]
    pub inputs: Vec<String>,
    /// Sugar: single default binding `input`
    #[arg(short = 'i', long = "input-file")]
    pub input_file: Option<PathBuf>,
    /// Recipe path (repeatable; required)
    #[arg(short = 'r', long = "recipe")]
    pub recipes: Vec<PathBuf>,
    #[arg(short = 'd', long = "output-dir")]
    pub output_dir: Option<PathBuf>,
    #[arg(long = "provider")]
    pub provider: Option<String>,
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,
    /// Variable: key=value (repeatable)
    #[arg(long = "var")]
    pub vars: Vec<String>,
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
    if cli.recipes.is_empty() {
        return Err(CliError::usage("no recipes specified"));
    }
    Ok(RunArgs {
        inputs: cli.inputs,
        input_file: cli.input_file,
        recipes: cli.recipes,
        output_dir: cli.output_dir,
        provider: cli.provider,
        model: cli.model,
        vars: cli.vars,
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        quiet: cli.quiet,
        overwrite: cli.overwrite,
    })
}
