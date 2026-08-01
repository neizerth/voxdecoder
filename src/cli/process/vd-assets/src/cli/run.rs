//! `vd-assets run`.

use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;

use super::{CliError, ProgressMode};
use crate::config;
use crate::convert::{self, ConvertRequest, OcrMode};
use crate::progress::{Progress, ProgressEvent};

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: Vec<PathBuf>,
    pub output: PathBuf,
    pub ocr: bool,
    pub force: bool,
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

#[derive(Debug, Serialize)]
struct DryRunPlan {
    inputs: Vec<String>,
    output: String,
    ocr: bool,
    force: bool,
    markdown_dir: String,
    terms: String,
}

pub fn execute(args: RunArgs) -> Result<(), CliError> {
    let _file = config::load(&crate::paths::config_path()).map_err(CliError::usage)?;

    let plan = DryRunPlan {
        inputs: args.input.iter().map(|p| p.display().to_string()).collect(),
        output: args.output.display().to_string(),
        ocr: args.ocr,
        force: args.force,
        markdown_dir: args.output.join("md").display().to_string(),
        terms: args.output.join("terms.yml").display().to_string(),
    };

    for p in &args.input {
        if !p.exists() {
            return Err(CliError::with_code(
                3,
                format!("input missing: {}", p.display()),
            ));
        }
    }

    if args.dry_run {
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&plan)
                    .map_err(|e| CliError::with_code(1, e.to_string()))?
            );
        } else {
            println!(
                "Inputs: {}\nOutput: {}\nMarkdown dir: {}\nTerms: {}\nOCR: {}\nForce: {}",
                plan.inputs.join(", "),
                plan.output,
                plan.markdown_dir,
                plan.terms,
                if plan.ocr { "on" } else { "off" },
                if plan.force { "on" } else { "off" },
            );
        }
        return Ok(());
    }

    let progress = Progress::new(args.effective_progress());
    let input_s = plan.inputs.join(", ");
    let out_s = args.output.display().to_string();
    progress.emit(&ProgressEvent::Start {
        input: Some(&input_s),
        output: Some(&out_s),
        artifact_type: Some("assets"),
        language: None,
        model: None,
        device: None,
        path: None,
    });

    let started = Instant::now();
    progress.emit(&ProgressEvent::phase("converting", 10));

    let result = convert::run(&ConvertRequest {
        inputs: args.input.clone(),
        output_dir: args.output.clone(),
        ocr: OcrMode::from_flag(args.ocr),
        force: args.force,
    })
    .map_err(|e| {
        progress.emit(&ProgressEvent::Error {
            code: "convert_failed",
            message: &e.to_string(),
        });
        CliError::with_code(e.exit_code(), e.to_string())
    })?;

    progress.emit(&ProgressEvent::phase("terms", 80));

    let duration_sec = started.elapsed().as_secs_f64();
    let terms_s = result.terms_path.display().to_string();
    let md_s = result.markdown_dir.display().to_string();
    progress.emit(&ProgressEvent::Done {
        output: Some(&terms_s),
        model: None,
        path: Some(&md_s),
        duration_sec: Some(duration_sec),
        char_count: Some(result.dictionary.forms.len()),
    });

    println!(
        "Assets: {}\nMarkdown: {}\nTerms: {}\nConverted: {}\nForms: {}",
        args.output.display(),
        result.markdown_dir.display(),
        result.terms_path.display(),
        result.converted.len(),
        result.dictionary.forms.len(),
    );

    Ok(())
}
