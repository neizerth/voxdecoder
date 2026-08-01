//! Job Executor.

mod bind;
mod subprocess;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use vd_progress::{Progress, ProgressMode};

use crate::job::{resolve, ArgValue, ResolvedJob};
use crate::status;

pub use bind::{Binder, InvokeRequest, InvokeResult};
pub use subprocess::SubprocessBinder;

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("{0}")]
    Step(String),
    #[error("{0}")]
    Reserved(String),
    #[error("{0}")]
    Other(String),
}

impl ExecError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Reserved(_) => 2,
            Self::Step(_) | Self::Other(_) => 1,
        }
    }
}

pub struct Executor<B: Binder> {
    pub binder: B,
    pub progress: ProgressMode,
}

impl<B: Binder> Executor<B> {
    pub fn run(&self, resolved: &ResolvedJob) -> Result<PathBuf, ExecError> {
        let progress = Progress::new(self.progress);
        let total = resolved.steps.len() as u32;
        let audio = resolved.job.input.audio.as_deref();
        let model = status::engine_from_steps(&resolved.steps);
        status::emit_start(&progress, audio, model.as_deref());

        let started = Instant::now();
        let mut artifacts = HashMap::new();
        let mut prev: Option<PathBuf> = None;
        let mut completed = 0u32;
        let mut last_out = None;
        let continue_on_error = resolved.job.continue_on_error;

        for (i, step) in resolved.steps.iter().enumerate() {
            let overall = status::overall_percent(completed, total);
            let job_step = &resolved.job.steps[i];

            if step.skip {
                status::emit_step_skipped(&progress, step, total, overall);
                completed += 1;
                continue;
            }

            let input = resolve::exec_input(
                job_step,
                &resolved.job,
                &resolved.working_dir,
                &artifacts,
                prev.as_ref(),
            )
            .map_err(|e| ExecError::Step(e.to_string()))?;

            let mut live = step.clone();
            live.input = Some(input.clone());
            status::emit_step_start(&progress, &live, total, overall);

            let req = InvokeRequest {
                capability: step.capability,
                working_dir: resolved.working_dir.clone(),
                input,
                output: step.output.clone(),
                output_dir: resolved.job.output.dir.clone(),
                context_assets: resolved.job.context.assets.clone(),
                options: step.options.clone(),
            };

            match self.binder.invoke(&req) {
                Ok(result) => {
                    if let Some(id) = &step.id {
                        artifacts.insert(id.clone(), result.primary_output.clone());
                    }
                    prev = Some(result.primary_output.clone());
                    last_out = Some(result.primary_output.clone());
                    completed += 1;
                    let overall_done = status::overall_percent(completed, total);
                    status::emit_step_done(
                        &progress,
                        &live,
                        total,
                        overall_done,
                        &result.primary_output,
                    );
                }
                Err(e) => {
                    status::emit_error(&progress, "step_failed", &e.to_string());
                    if !continue_on_error {
                        return Err(e);
                    }
                    completed += 1;
                }
            }
        }

        let out = last_out.ok_or_else(|| ExecError::Other("job produced no output".into()))?;
        status::emit_done(&progress, Some(&out), started.elapsed().as_secs_f64());
        Ok(out)
    }
}

pub fn dry_run_text(resolved: &ResolvedJob) -> String {
    let mut lines = Vec::new();
    let name = resolved.job.name.as_deref().unwrap_or("(unnamed)");
    lines.push(format!("Job: {name}"));
    lines.push(format!("working_dir: {}", resolved.working_dir.display()));
    if let Some(a) = &resolved.job.input.audio {
        lines.push(format!("input.audio: {}", a.display()));
    }
    lines.push(format!("steps: {}", resolved.steps.len()));
    for (i, s) in resolved.steps.iter().enumerate() {
        let job_step = &resolved.job.steps[i];
        let mut parts = vec![format!("{}. {}", s.index, s.capability.as_str())];
        if s.skip {
            parts.push("skip".into());
        }
        if let Some(id) = &s.id {
            parts.push(format!("id={id}"));
        }
        if let Some(inp) = &job_step.input {
            parts.push(format!("input={inp}"));
        } else if let Some(inp) = &s.input {
            parts.push(format!("input={}", inp.display()));
        } else {
            parts.push("input=<prev>".into());
        }
        if let Some(engine) = s.options.get("engine").and_then(ArgValue::as_string) {
            parts.push(format!("engine={engine}"));
        }
        if let Some(model) = s.options.get("model").and_then(ArgValue::as_string) {
            parts.push(format!("model={model}"));
        }
        lines.push(format!("  {}", parts.join("  ")));
    }
    lines.join("\n")
}

pub fn dry_run_json(resolved: &ResolvedJob) -> Result<String, ExecError> {
    serde_json::to_string_pretty(&resolved.job).map_err(|e| ExecError::Other(e.to_string()))
}
