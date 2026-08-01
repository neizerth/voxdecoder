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

use crate::config::resolve::{Device, OutputFormat};
use crate::platform;
pub use crate::progress::ProgressMode;

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
    name = "vd-giga",
    version,
    about = "Local GigaAM transcription CLI (Candle, no Python at runtime)",
    long_about = "Transcribe audio/video with GigaAM models in-process.\n\n\
Shorthand: vd-giga -i FILE  ≡  vd-giga run -i FILE\n\n\
Models are installed under the managed cache (or --download-root / VD_GIGA_MODELS_DIR)."
)]
struct Root {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Transcribe a local audio or video file
    Run(RunCli),
    /// Show or change default settings
    Config(ConfigArgs),
    /// Download a catalog GigaAM model (and convert to SafeTensors when possible)
    Install(InstallArgs),
    /// Remove an installed model
    Remove(RemoveArgs),
    /// List installed (or all catalog) models
    List(ListArgs),
    /// Show model metadata without loading weights onto the GPU
    Info(InfoArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Transcribe a local audio or video file",
    after_help = "Examples:\n  \
vd-giga run -i meeting.ogg\n  \
vd-giga -i meeting.ogg -m v3_e2e_ctc --dry-run\n  \
vd-giga run -i call.wav -o out.txt --format json --segments"
)]
pub struct RunCli {
    /// Path to audio or video
    #[arg(short = 'i', long = "input", required = true)]
    pub input: PathBuf,
    /// Explicit output file path (mutually exclusive with --output-dir)
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    /// Write `{stem}.{ext}` into this directory
    #[arg(short = 'd', long = "output-dir")]
    pub output_dir: Option<PathBuf>,
    /// Output format [default: txt]
    #[arg(long = "format", value_enum)]
    pub format: Option<CliFormat>,
    /// Also write `{stem}.segments.json` next to the main output
    #[arg(long = "segments")]
    pub segments: bool,
    /// Replace existing output files (default: error if present)
    #[arg(long = "overwrite")]
    pub overwrite: bool,
    /// Print resolved options and exit (no transcription)
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// With --dry-run: print the plan as JSON on stdout
    #[arg(long = "json")]
    pub json: bool,
    /// Progress on stderr: text|json (off if omitted)
    #[arg(long = "progress", value_enum)]
    pub progress: Option<CliProgress>,
    /// Catalog name or path to weights (default: v2_rnnt / config)
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,
    /// Inference device for this build
    #[arg(
        long = "device",
        value_parser = PossibleValuesParser::new(Device::allowed()),
        help = DEVICE_HELP
    )]
    pub device: Option<String>,
    /// Disable FP16 encoder (default: FP16 on for GPU paths)
    #[arg(long = "no-fp16-encoder")]
    pub no_fp16_encoder: bool,
    /// Checkpoint / converted-model directory (default: managed cache)
    #[arg(long = "download-root")]
    pub download_root: Option<PathBuf>,
    /// Request word-level timestamps (requires --format json and/or --segments)
    #[arg(long = "word-timestamps")]
    pub word_timestamps: bool,

    /// Enable FlashAttention (CUDA builds only)
    #[cfg(not(target_os = "macos"))]
    #[arg(long = "flash")]
    pub flash: bool,
}

#[cfg(all(target_os = "macos", feature = "metal"))]
const DEVICE_HELP: &str = "Inference device [cpu|metal|auto] (default: auto)";
#[cfg(all(target_os = "macos", not(feature = "metal")))]
const DEVICE_HELP: &str = "Inference device [cpu|auto] (default: auto)";
#[cfg(not(target_os = "macos"))]
const DEVICE_HELP: &str = "Inference device [cpu|cuda|auto] (default: auto)";

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum CliFormat {
    Txt,
    Json,
    Srt,
    Vtt,
}

impl From<CliFormat> for OutputFormat {
    fn from(v: CliFormat) -> Self {
        match v {
            CliFormat::Txt => Self::Txt,
            CliFormat::Json => Self::Json,
            CliFormat::Srt => Self::Srt,
            CliFormat::Vtt => Self::Vtt,
        }
    }
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
    /// Print all keys (merged with defaults)
    List,
    /// Print one key
    Get {
        /// Config key (model|device|fp16_encoder|flash|download_root|word_timestamps|format)
        key: String,
    },
    /// Set one key (booleans: on|off)
    Set {
        key: String,
        value: String,
    },
    /// Print path to config.toml
    Path,
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Download a catalog GigaAM model",
    long_about = "Download a GigaAM checkpoint from the official CDN into the models directory,\n\
then convert to SafeTensors when convert_ckpt.py (or VD_GIGA_CONVERT_SCRIPT) is available.\n\n\
Pass a catalog MODEL name, a short alias, or --all.",
    after_help = crate::gigaam::catalog::INSTALL_HELP
)]
pub struct InstallArgs {
    /// Catalog model name or alias (see after help for the full list)
    #[arg(value_name = "MODEL")]
    pub model: Option<String>,
    /// Install every catalog model
    #[arg(long = "all")]
    pub all: bool,
    /// Checkpoint directory (same as run --download-root)
    #[arg(long = "download-root")]
    pub download_root: Option<PathBuf>,
    /// Progress on stderr: text|json (off if omitted)
    #[arg(long = "progress", value_enum)]
    pub progress: Option<CliProgress>,
}

#[derive(Debug, Clone, Parser)]
#[command(about = "Remove an installed model")]
pub struct RemoveArgs {
    /// Catalog name or path
    pub model: String,
    /// Do not prompt for confirmation
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

#[derive(Debug, Clone, Parser)]
#[command(about = "List models")]
pub struct ListArgs {
    /// Include catalog models that are not installed
    #[arg(long = "all")]
    pub all: bool,
    /// Machine-readable list on stdout
    #[arg(long = "json")]
    pub json: bool,
}

#[derive(Debug, Clone, Parser)]
#[command(about = "Show model metadata")]
pub struct InfoArgs {
    /// Catalog name or local checkpoint path
    pub model: String,
    /// Machine-readable metadata on stdout
    #[arg(long = "json")]
    pub json: bool,
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

    pub fn exit_code(&self) -> u8 {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn with_code(code: u8, msg: impl AsRef<str>) -> Self {
        Self {
            code,
            message: msg.as_ref().to_string(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

/// Parse argv into a validated command.
pub fn parse_args<I, T>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let raw: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let normalized = normalize_argv(raw);

    let root = Root::try_parse_from(&normalized).map_err(|e| {
        let _ = e.print();
        CliError::usage("")
    })?;

    match root.command {
        RootCommand::Run(cli) => Ok(Command::Run(validate_run(cli)?)),
        RootCommand::Config(a) => Ok(Command::Config(a)),
        RootCommand::Install(a) => {
            if a.model.is_none() && !a.all {
                return Err(CliError::usage(format!(
                    "install requires MODEL or --all\n\n{}",
                    crate::gigaam::catalog::INSTALL_HELP
                )));
            }
            if let Some(ref m) = a.model {
                if !a.all && !crate::gigaam::catalog::is_catalog_name(m) {
                    return Err(CliError::usage(format!(
                        "unknown model '{m}'\n\n{}",
                        crate::gigaam::catalog::INSTALL_HELP
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

fn validate_run(cli: RunCli) -> Result<RunArgs, CliError> {
    if cli.output.is_some() && cli.output_dir.is_some() {
        return Err(CliError::usage(
            "--output and --output-dir are mutually exclusive",
        ));
    }

    let format = cli.format.map(OutputFormat::from);
    if cli.word_timestamps {
        let sink_ok = matches!(format, Some(OutputFormat::Json)) || cli.segments;
        if !sink_ok {
            return Err(CliError::usage(
                "--word-timestamps requires --format json or --segments",
            ));
        }
    }

    let device = match cli.device.as_deref() {
        None => None,
        Some(s) => Some(Device::parse(s).ok_or_else(|| {
            CliError::usage(format!(
                "invalid --device '{s}' (expected {})",
                Device::allowed().join("|")
            ))
        })?),
    };

    #[cfg(not(target_os = "macos"))]
    let flash = cli.flash;
    #[cfg(target_os = "macos")]
    let flash = false;
    let _ = platform::FLASH_SUPPORTED;

    Ok(RunArgs {
        input: cli.input,
        output: cli.output,
        output_dir: cli.output_dir,
        format,
        segments: cli.segments,
        overwrite: cli.overwrite,
        dry_run: cli.dry_run,
        json: cli.json,
        progress: cli.progress.map(ProgressMode::from),
        model: cli.model,
        device,
        no_fp16_encoder: cli.no_fp16_encoder,
        flash,
        download_root: cli.download_root,
        word_timestamps: cli.word_timestamps,
    })
}

fn normalize_argv(args: Vec<OsString>) -> Vec<OsString> {
    if args.len() < 2 {
        return args;
    }
    let first = args[1].as_os_str();
    if is_subcommand(first) {
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
