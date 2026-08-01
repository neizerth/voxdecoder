//! `vd-fix-asr run` implementation.

use std::path::PathBuf;
use std::time::Instant;

use super::{CliError, ProgressMode};
use crate::artifact::{self, count_text_spans};
use crate::asr::{AsrFixer, AsrLoadOptions};
use crate::config::{self, resolve_run, RunOverrides};
use crate::context::visit_text_spans;
use crate::progress::{Progress, ProgressEvent};
use crate::types::{FixOptions, Language, ProgressFormat, TextSpan};

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub overwrite: bool,
    pub language: Option<Language>,
    pub context: Vec<PathBuf>,
    pub context_neighbors: Option<u32>,
    pub dry_run: bool,
    pub json: bool,
    pub progress: Option<ProgressMode>,
    pub quiet: bool,
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

#[allow(clippy::too_many_lines)]
pub fn execute(args: RunArgs) -> Result<(), CliError> {
    if !args.input.exists() {
        return Err(CliError::with_code(
            3,
            format!("input file missing / unreadable: {}", args.input.display()),
        ));
    }

    let artifact_type = artifact::detect_type(&args.input)
        .ok_or_else(|| CliError::with_code(3, "unsupported artifact type"))?;

    let file = config::load(&crate::paths::config_path()).map_err(CliError::usage)?;

    let progress_fmt = match args.effective_progress() {
        ProgressMode::Text => Some(ProgressFormat::Text),
        ProgressMode::Json => Some(ProgressFormat::Json),
        ProgressMode::None => None,
    };

    let resolved = resolve_run(
        args.input.clone(),
        artifact_type,
        &file,
        RunOverrides {
            language: args.language,
            in_place: None,
            progress: progress_fmt,
            output: args.output.clone(),
            output_dir: args.output_dir.clone(),
            overwrite: args.overwrite,
            cli_in_place: args.in_place,
            context: args.context.clone(),
            context_neighbors: args.context_neighbors,
        },
    )
    .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;

    if args.dry_run {
        if args.json {
            let plan = resolved.dry_run_plan();
            println!(
                "{}",
                serde_json::to_string_pretty(&plan)
                    .map_err(|e| CliError::with_code(1, e.to_string()))?
            );
        } else {
            println!("{}", resolved.dry_run_text());
        }
        return Ok(());
    }

    let progress = Progress::new(args.effective_progress());
    let input_s = resolved.input.display().to_string();
    let output_s = resolved.paths.main.display().to_string();
    progress.emit(&ProgressEvent::Start {
        input: Some(&input_s),
        output: Some(&output_s),
        artifact_type: Some(resolved.artifact_type.as_str()),
        language: Some(resolved.language.as_str()),
        model: None,
        device: None,
        path: None,
    });

    let started = Instant::now();
    progress.emit(&ProgressEvent::phase("loading", 5));

    let mut artifact = artifact::load(&resolved.input).map_err(|e| {
        progress.emit(&ProgressEvent::Error {
            code: "load_failed",
            message: &e.to_string(),
        });
        CliError::with_code(e.exit_code(), e.to_string())
    })?;

    let fixer = AsrFixer::load(AsrLoadOptions {
        language: resolved.language,
        context_paths: resolved.context.clone(),
        neighbor_window: resolved.context_neighbors,
    })
    .map_err(|e| {
        progress.emit(&ProgressEvent::Error {
            code: "backend_init_failed",
            message: &e.to_string(),
        });
        CliError::with_code(e.exit_code(), e.to_string())
    })?;

    let span_total = count_text_spans(&artifact).max(1);
    let mut span_idx = 0u32;
    let mut char_count = 0usize;
    let materials = fixer.materials().clone();
    let neighbor_window = fixer.neighbor_window();
    visit_text_spans(
        &mut artifact,
        neighbor_window,
        &materials,
        |span: TextSpan<'_>, ctx| -> Result<(), CliError> {
            span_idx += 1;
            let percent = ((u64::from(span_idx) * 80) / span_total as u64) as u8 + 10;
            progress.emit(&ProgressEvent::phase_span(
                "processing",
                percent.min(90),
                span_idx,
                span_total as u32,
            ));
            let result = fixer
                .fix(&span, ctx, FixOptions::default())
                .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
            char_count += result.text.chars().count();
            if result.changed {
                *span.text = result.text;
            }
            Ok(())
        },
    )?;

    progress.emit(&ProgressEvent::phase("writing", 95));

    artifact::write(&artifact, &resolved.paths.main).map_err(|e| {
        progress.emit(&ProgressEvent::Error {
            code: "write_failed",
            message: &e.to_string(),
        });
        CliError::with_code(e.exit_code(), e.to_string())
    })?;

    let duration_sec = started.elapsed().as_secs_f64();
    progress.emit(&ProgressEvent::Done {
        output: Some(&output_s),
        model: None,
        path: None,
        duration_sec: Some(duration_sec),
        char_count: Some(char_count),
    });

    Ok(())
}
