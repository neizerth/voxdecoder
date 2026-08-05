//! `vd-pipeline run`.

use std::fs;
use std::path::{Path, PathBuf};

use super::{CliError, ProgressMode};
use crate::config;
use crate::exec::{self, Executor, SubprocessBinder};
use crate::job::{
    default_job, load_job_file, resolve_job, DefaultJobArgs, JobError, ResolvedJob,
    TranscribeEngine,
};
use crate::paths;
use crate::report::ExecutionReport;

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: Option<PathBuf>,
    pub job_file: Option<PathBuf>,
    pub asr: String,
    pub model: Option<String>,
    pub device: Option<String>,
    pub flash: bool,
    pub docs: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub dry_run: bool,
    pub json: bool,
    pub progress: Option<ProgressMode>,
    pub quiet: bool,
    pub continue_on_error: bool,
    pub overwrite: bool,
    pub report: Option<PathBuf>,
    pub report_dir: Option<PathBuf>,
    /// True if interactive mode is requested (explicit --interactive or auto-detected TTY)
    pub interactive: bool,
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

        // Interactive mode: classify and confirm single audio file
        let audio_to_use = if args.interactive && !args.dry_run {
            classify_and_confirm_audio(&audio)?
        } else {
            audio.clone()
        };
        let asr = if args.asr == d.asr {
            file.asr.clone().unwrap_or_else(|| d.asr.to_string())
        } else {
            args.asr.clone()
        };
        let engine = TranscribeEngine::parse(&asr)
            .ok_or_else(|| CliError::usage(format!("invalid --asr: {}", args.asr)))?;
        default_job(&DefaultJobArgs {
            audio: audio_to_use,
            engine,
            model: args.model.clone(),
            device: args.device.clone(),
            flash: args.flash,
            speed: None,
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
        progress_snapshot: None,
    };
    match executor.run(&resolved) {
        Ok(outcome) => {
            write_reports(&args, &resolved, &outcome.report)?;
            println!("output: {}", outcome.output.display());
            Ok(())
        }
        Err(fail) => {
            let _ = write_reports(&args, &resolved, &fail.report);
            Err(CliError::with_code(fail.exit_code(), fail.to_string()))
        }
    }
}

fn write_reports(
    args: &RunArgs,
    resolved: &ResolvedJob,
    report: &ExecutionReport,
) -> Result<(), CliError> {
    if let Some(path) = &args.report {
        report
            .write_to(path)
            .map_err(|e| CliError::with_code(1, format!("write report: {e}")))?;
        return Ok(());
    }
    if let Some(dir) = &args.report_dir {
        fs::create_dir_all(dir).map_err(|e| CliError::with_code(1, e.to_string()))?;
        report
            .write_to(&dir.join("report.json"))
            .map_err(|e| CliError::with_code(1, format!("write report: {e}")))?;
        write_resolved_job(dir, resolved)?;
    }
    Ok(())
}

fn write_resolved_job(dir: &Path, resolved: &ResolvedJob) -> Result<(), CliError> {
    let body = serde_json::to_string_pretty(&resolved.job)
        .map_err(|e| CliError::with_code(1, e.to_string()))?;
    fs::write(dir.join("resolved-job.json"), body)
        .map_err(|e| CliError::with_code(1, e.to_string()))
}

fn map_job_err(e: JobError) -> CliError {
    CliError::with_code(e.exit_code(), e.to_string())
}

/// Classify single audio file and show confirm menu to user.
pub fn classify_and_confirm_audio(audio: &Path) -> Result<PathBuf, CliError> {
    use std::io::{BufRead, Write};

    // Classify the single file
    let classified = vd_classify::classify_inputs(&[audio.to_path_buf()]);
    if classified.is_empty() {
        return Ok(audio.to_path_buf());
    }

    let c = &classified[0];
    let gender_str = match c.gender {
        Some(g) => format!("{:?}", g),
        None => "?".to_string(),
    };

    eprintln!("Detected: {:?} {} [{}]", c.role, c.name, gender_str);

    // Show simple y/n confirm (lightweight version for audio)
    eprint!("Proceed? (y/N): ");
    std::io::stderr().flush().ok();

    let stdin = std::io::stdin();
    let mut buf = String::new();
    stdin
        .lock()
        .read_line(&mut buf)
        .map_err(|e| CliError::usage(format!("read stdin: {}", e)))?;

    if !buf.trim().eq_ignore_ascii_case("y") {
        return Err(CliError::usage("audio confirmation aborted".to_string()));
    }

    Ok(audio.to_path_buf())
}

#[cfg(test)]
mod interactive_tests {
    use super::*;

    #[test]
    fn non_interactive_preserves_input() {
        // Non-interactive mode should not call classify_and_confirm_audio
        // and should use the input path unchanged
        let run_args = RunArgs {
            input: Some(PathBuf::from("test.wav")),
            job_file: None,
            asr: "whisper".to_string(),
            model: None,
            device: None,
            flash: false,
            docs: None,
            output_dir: None,
            working_dir: None,
            dry_run: true,
            json: false,
            progress: None,
            quiet: false,
            continue_on_error: false,
            overwrite: false,
            report: None,
            report_dir: None,
            interactive: false, // Non-interactive
        };

        // With dry_run, we just verify the args are preserved
        assert_eq!(run_args.input, Some(PathBuf::from("test.wav")));
        assert!(!run_args.interactive);
    }

    #[test]
    fn interactive_flag_set() {
        // Verify that interactive flag can be set on RunArgs
        let run_args = RunArgs {
            input: Some(PathBuf::from("test.wav")),
            job_file: None,
            asr: "whisper".to_string(),
            model: None,
            device: None,
            flash: false,
            docs: None,
            output_dir: None,
            working_dir: None,
            dry_run: false,
            json: false,
            progress: None,
            quiet: false,
            continue_on_error: false,
            overwrite: false,
            report: None,
            report_dir: None,
            interactive: true, // Interactive mode enabled
        };

        assert!(run_args.interactive);
        assert_eq!(run_args.input, Some(PathBuf::from("test.wav")));
    }
}
