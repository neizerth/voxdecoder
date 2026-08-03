//! `vd-gigaam run` — transcribe or `--dry-run`.

use std::path::PathBuf;

use crate::cli::CliError;
use crate::config::file as config_file;
use crate::config::resolve::{self, Device, DryRunPlan, OutputFormat, RunOverrides};
use crate::gigaam::config::{GigaLoadOptions, TranscribeOptions};
use crate::gigaam::model::{GigaModel, ModelError};
use crate::output::OutputPathError;

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

    let progress = Progress::from_env(args.effective_progress());
    progress.emit(&ProgressEvent::Start {
        input: Some(resolved.input.to_str().unwrap_or("")),
        output: Some(resolved.plan.output.to_str().unwrap_or("")),
        artifact_type: None,
        language: None,
        model: Some(&resolved.plan.model),
        device: Some(resolved.plan.device.as_str()),
        path: None,
    });

    let samples = crate::audio::load_pcm16k_mono(&resolved.input)
        .map_err(|e| CliError::with_code(1, e.to_string()))?;

    let load_opts = |device: resolve::Device| GigaLoadOptions {
        model: resolved.plan.model.clone(),
        device,
        fp16_encoder: resolved.plan.fp16_encoder,
        flash: resolved.plan.flash,
        download_root: resolved.plan.download_root.clone(),
    };
    let tx_opts = TranscribeOptions {
        word_timestamps: resolved.plan.word_timestamps,
    };

    let allow_cpu_fallback = resolved.plan.device != resolve::Device::Cpu;
    progress.emit(&ProgressEvent::phase("loading_model", 5));
    let mut model = match GigaModel::load(load_opts(resolved.plan.device)) {
        Ok(m) => m,
        Err(err)
            if allow_cpu_fallback
                && crate::metal_fallback::is_metal_resource_error(&err.to_string()) =>
        {
            eprintln!("warning: Metal GPU resource failed ({err}); retrying on CPU");
            progress.emit(&ProgressEvent::phase("loading_model", 5));
            GigaModel::load(load_opts(resolve::Device::Cpu)).map_err(map_model_err)?
        }
        Err(err) => return Err(map_model_err(err)),
    };

    let emit_chunk = |done: u32, total: u32| {
        let pct = if total == 0 {
            55
        } else {
            10 + ((done.saturating_mul(85)) / total).min(85) as u8
        };
        progress.emit(&ProgressEvent::phase_segment(
            "transcribing",
            pct,
            done,
            total,
        ));
    };

    progress.emit(&ProgressEvent::phase_segment("transcribing", 10, 0, 1));
    let transcript = match model.transcribe_with_progress(
        &samples,
        tx_opts.clone(),
        |d, t| emit_chunk(d, t),
    ) {
        Ok(t) => t,
        Err(err)
            if allow_cpu_fallback
                && crate::metal_fallback::is_metal_resource_error(&err.to_string()) =>
        {
            eprintln!("warning: Metal GPU resource failed ({err}); retrying on CPU");
            drop(model);
            progress.emit(&ProgressEvent::phase("loading_model", 5));
            model = GigaModel::load(load_opts(resolve::Device::Cpu)).map_err(map_model_err)?;
            progress.emit(&ProgressEvent::phase_segment("transcribing", 10, 0, 1));
            model
                .transcribe_with_progress(&samples, tx_opts, |d, t| emit_chunk(d, t))
                .map_err(map_model_err)?
        }
        Err(err) => return Err(map_model_err(err)),
    };
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
