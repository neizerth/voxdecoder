//! `vd-giga run` — transcribe or `--dry-run`.

use std::path::PathBuf;

use crate::cli::CliError;
use crate::config::file as config_file;
use crate::config::resolve::{self, Device, DryRunPlan, OutputFormat, RunOverrides};
use crate::gigaam::config::{GigaLoadOptions, TranscribeOptions};
use crate::gigaam::model::{GigaModel, ModelError};
use crate::output::path::OutputPathError;
use crate::output::writer;
use crate::paths;
use crate::progress::{Progress, ProgressEvent, ProgressMode};

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub format: Option<OutputFormat>,
    pub segments: bool,
    pub overwrite: bool,
    pub dry_run: bool,
    pub json: bool,
    pub progress: Option<ProgressMode>,
    pub quiet: bool,
    pub model: Option<String>,
    pub device: Option<Device>,
    pub no_fp16_encoder: bool,
    pub flash: bool,
    pub download_root: Option<PathBuf>,
    pub word_timestamps: bool,
}

impl RunArgs {
    pub fn effective_progress(&self) -> ProgressMode {
        if self.quiet {
            ProgressMode::None
        } else {
            self.progress.unwrap_or(ProgressMode::Text)
        }
    }
}

pub fn execute(args: RunArgs) -> Result<(), CliError> {
    if !args.input.is_file() {
        return Err(CliError::with_code(
            3,
            format!("input file missing or unreadable: {}", args.input.display()),
        ));
    }

    // Re-check word_timestamps against merged format (config may set json).
    let file_cfg = config_file::load(&paths::config_path()).map_err(CliError::usage)?;
    let resolved = resolve::resolve_run(
        &file_cfg,
        RunOverrides {
            input: args.input.clone(),
            output: args.output.clone(),
            output_dir: args.output_dir.clone(),
            format: args.format,
            segments: args.segments,
            overwrite: args.overwrite,
            model: args.model.clone(),
            device: args.device,
            no_fp16_encoder: args.no_fp16_encoder,
            flash: args.flash,
            download_root: args.download_root.clone(),
            word_timestamps: args.word_timestamps,
        },
    )
    .map_err(map_path_err)?;

    if args.word_timestamps {
        let sink_ok =
            matches!(resolved.format, OutputFormat::Json) || resolved.plan.segments.is_some();
        if !sink_ok {
            return Err(CliError::usage(
                "--word-timestamps requires --format json or --segments",
            ));
        }
    }

    if args.dry_run {
        print_dry_run(&resolved.plan, args.json);
        return Ok(());
    }

    let progress = Progress::new(args.effective_progress());
    progress.emit(&ProgressEvent::Start {
        input: Some(resolved.input.to_str().unwrap_or("")),
        output: Some(resolved.plan.output.to_str().unwrap_or("")),
        model: Some(&resolved.plan.model),
        device: Some(resolved.plan.device.as_str()),
        path: None,
    });

    progress.emit(&ProgressEvent::Phase {
        phase: "loading_model",
        percent: 5,
        segment: None,
        segment_total: None,
        bytes_done: None,
        bytes_total: None,
    });

    let model = GigaModel::load(GigaLoadOptions {
        model: resolved.plan.model.clone(),
        device: resolved.plan.device,
        fp16_encoder: resolved.plan.fp16_encoder,
        flash: resolved.plan.flash,
        download_root: resolved.plan.download_root.clone(),
    })
    .map_err(map_model_err)?;

    let samples = crate::audio::load_pcm16k_mono(&resolved.input)
        .map_err(|e| CliError::with_code(1, e.to_string()))?;

    progress.emit(&ProgressEvent::Phase {
        phase: "transcribing",
        percent: 55,
        segment: None,
        segment_total: None,
        bytes_done: None,
        bytes_total: None,
    });

    let transcript = model
        .transcribe(
            &samples,
            TranscribeOptions {
                word_timestamps: resolved.plan.word_timestamps,
            },
        )
        .map_err(map_model_err)?;

    writer::write_outputs(
        &resolved.plan.output,
        resolved.plan.segments.as_deref(),
        resolved.format,
        &transcript,
    )
    .map_err(|e| CliError::with_code(1, e.to_string()))?;

    progress.emit(&ProgressEvent::Done {
        output: Some(resolved.plan.output.to_str().unwrap_or("")),
        model: None,
        path: None,
        duration_sec: None,
        char_count: Some(transcript.text.chars().count()),
    });

    Ok(())
}

fn print_dry_run(plan: &DryRunPlan, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(plan).unwrap_or_default());
        return;
    }
    println!("Model: {}", plan.model);
    println!("Device: {}", plan.device.as_str());
    println!("Flash: {}", on_off(plan.flash));
    println!("FP16 encoder: {}", on_off(plan.fp16_encoder));
    println!("Download root: {}", plan.download_root.display());
    println!("Output: {}", plan.output.display());
    if let Some(seg) = &plan.segments {
        println!("Segments: {}", seg.display());
    }
    println!("Overwrite: {}", on_off(plan.overwrite));
    println!("Word timestamps: {}", on_off(plan.word_timestamps));
}

fn on_off(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}

fn map_path_err(err: OutputPathError) -> CliError {
    CliError::with_code(err.exit_code(), err.to_string())
}

fn map_model_err(err: ModelError) -> CliError {
    match &err {
        ModelError::Weights(_) | ModelError::Load(_) => CliError::with_code(4, err.to_string()),
        ModelError::Transcribe(_) => CliError::with_code(1, err.to_string()),
    }
}
