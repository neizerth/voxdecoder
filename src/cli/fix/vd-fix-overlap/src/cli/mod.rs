//! Commands from `cli.md`.

mod config_cmd;
mod run;

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub use run::RunArgs;

#[derive(Debug)]
pub enum Command {
    Run(RunArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "vd-fix-overlap",
    version,
    about = "Remove duplicated speech introduced by diarization overlap",
    long_about = "Never deletes unique speech. Only removes duplicated content.\n\n\
Shorthand: vd-fix-overlap -i FILE  ≡  vd-fix-overlap run -i FILE\n\n\
Reads a diarized JSON/JSONL transcript (speaker + start_sec + end_sec + text per turn),\n\
reports candidate duplicate pairs, and — with --apply (or any output flag) — removes the\n\
`drop` side of each pair and writes a fixed artifact."
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Detect (and optionally remove) duplicated speech across speakers
    Run(RunCli),
    /// Show or change default detection thresholds
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Detect (and optionally remove) duplicated speech across speakers",
    after_help = "Examples:\n  \
vd-fix-overlap run -i meeting.json\n  \
vd-fix-overlap -i meeting.json --json\n  \
vd-fix-overlap run -i meeting.json --similarity-threshold 0.9 --max-gap-ms 250\n  \
vd-fix-overlap run -i meeting.json --apply\n  \
vd-fix-overlap run -i meeting.json -o cleaned.json"
)]
pub struct RunCli {
    /// Diarized transcript: JSON/JSONL turns with speaker + start_sec + end_sec + text
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
    /// Remove the `drop` side of every detected pair and write a fixed
    /// artifact. Implied by `--output` / `--output-dir` / `--in-place`.
    #[arg(long = "apply")]
    pub apply: bool,
    #[arg(long = "similarity-threshold")]
    pub similarity_threshold: Option<f64>,
    #[arg(long = "max-gap-ms")]
    pub max_gap_ms: Option<u64>,
    /// Machine-readable report on stdout instead of the text summary
    #[arg(long = "json")]
    pub json: bool,
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
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
    if let Some(t) = cli.similarity_threshold {
        if !(0.0..=1.0).contains(&t) {
            return Err(CliError::usage(format!(
                "--similarity-threshold must be in [0.0, 1.0], got {t}"
            )));
        }
    }
    let targets = u8::from(cli.output.is_some())
        + u8::from(cli.output_dir.is_some())
        + u8::from(cli.in_place);
    if targets > 1 {
        return Err(CliError::usage(
            "--output, --output-dir, and --in-place are mutually exclusive",
        ));
    }
    let apply = cli.apply || targets > 0;
    Ok(RunArgs {
        input: cli.input,
        output: cli.output,
        output_dir: cli.output_dir,
        in_place: cli.in_place,
        overwrite: cli.overwrite,
        apply,
        similarity_threshold: cli.similarity_threshold,
        max_gap_ms: cli.max_gap_ms,
        json: cli.json,
        quiet: cli.quiet,
    })
}
