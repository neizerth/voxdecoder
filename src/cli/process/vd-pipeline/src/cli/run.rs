//! `vd-pipeline run`.

use std::path::PathBuf;

use super::{CliError, ProgressMode};
use crate::config;
use crate::exec::{self, Executor, SubprocessBinder};
use crate::job::{
    default_job, load_job_file, resolve_job, DefaultJobArgs, JobError, TranscribeEngine,
};
use crate::paths;

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: Option<PathBuf>,
    pub job_file: Option<PathBuf>,
    pub asr: String,
    pub model: Option<String>,
    pub docs: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub dry_run: bool,
    pub json: bool,
    pub progress: Option<ProgressMode>,
    pub quiet: bool,
    pub continue_on_error: bool,
    pub overwrite: bool,
}

impl RunArgs {
    pub fn effective_progress(&self, file_progress: Option<&str>) -> ProgressMode {
        if self.quiet {
            return ProgressMode::None;
        }
        if let Some(p) = self.progress {
            return p;
        }
        file_progress
            .and_then(ProgressMode::parse)
            .unwrap_or(ProgressMode::Text)
    }
}

pub fn execute(args: RunArgs) -> Result<(), CliError> {
    let file = config::load(&paths::config_path()).map_err(CliError::usage)?;
    let d = config::defaults();

    let continue_on_error =
        args.continue_on_error || file.continue_on_error.unwrap_or(d.continue_on_error);

    let job = if let Some(path) = &args.job_file {
        if !path.exists() {
            return Err(CliError::with_code(
                3,
                format!("job file missing: {}", path.display()),
            ));
        }
        load_job_file(path).map_err(map_job_err)?
    } else {
        let audio = args
            .input
            .clone()
            .ok_or_else(|| CliError::with_code(3, "missing -i / --input".to_string()))?;
        if !audio.exists() {
            return Err(CliError::with_code(
                3,
                format!("input missing: {}", audio.display()),
            ));
        }
        let asr = if args.asr == d.asr {
            file.asr.clone().unwrap_or_else(|| d.asr.to_string())
        } else {
            args.asr.clone()
        };
        let engine = TranscribeEngine::parse(&asr)
            .ok_or_else(|| CliError::usage(format!("invalid --asr: {}", args.asr)))?;
        default_job(&DefaultJobArgs {
            audio,
            engine,
            model: args.model.clone(),
            docs: args.docs.clone(),
            output_dir: args.output_dir.clone(),
            working_dir: args.working_dir.clone(),
            continue_on_error,
            overwrite: args.overwrite,
        })
    };

    let mut job = job;
    if continue_on_error {
        job.continue_on_error = true;
    }

    let resolved = resolve_job(job).map_err(map_job_err)?;

    if args.dry_run {
        if args.json {
            println!(
                "{}",
                exec::dry_run_json(&resolved).map_err(|e| CliError::with_code(1, e.to_string()))?
            );
        } else {
            println!("{}", exec::dry_run_text(&resolved));
        }
        return Ok(());
    }

    let executor = Executor {
        binder: SubprocessBinder,
        progress: args.effective_progress(file.progress.as_deref()),
    };
    let out = executor
        .run(&resolved)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
    println!("output: {}", out.display());
    Ok(())
}

fn map_job_err(e: JobError) -> CliError {
    CliError::with_code(e.exit_code(), e.to_string())
}
