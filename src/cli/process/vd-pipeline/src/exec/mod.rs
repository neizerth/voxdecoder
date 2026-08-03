//! Job Executor — recursive workflow (`sequence` / `parallel`) over capability leaves.

mod bind;
mod resources;
mod subprocess;
mod timemap;

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant, SystemTime};

use vd_progress::{Progress, ProgressMode};

use crate::artifacts::ArtifactRegistry;
use crate::job::{resolve, ArgValue, ArtifactRef, Capability, ResolvedJob, WorkflowPlan};
use crate::report::{
    self, duration_ms, format_rfc3339, make_step_report, ExecutionReport, JobReportStatus,
    StepReport, StepReportStatus,
};
use crate::status;

use resources::{resolve_job_capacity, step_resource_need, ResourcePool};

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

#[derive(Debug, Clone)]
pub struct ExecOutcome {
    pub output: PathBuf,
    pub report: ExecutionReport,
}

#[derive(Debug)]
pub struct ExecFailure {
    pub error: ExecError,
    pub report: ExecutionReport,
}

impl ExecFailure {
    pub fn exit_code(&self) -> u8 {
        self.error.exit_code()
    }
}

impl fmt::Display for ExecFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for ExecFailure {}

pub struct Executor<B: Binder> {
    pub binder: B,
    pub progress: ProgressMode,
    /// When set, step progress is snapshotted here for Runtime observe (`get_job`).
    pub progress_snapshot: Option<PathBuf>,
}

struct RunState {
    artifacts: ArtifactRegistry,
    prev: Option<PathBuf>,
    last_out: Option<PathBuf>,
    /// Active TimeMap from the latest preprocess that rewrote time (ADR §5–6).
    active_timemap: Option<PathBuf>,
    step_reports: Vec<StepReport>,
    any_failed: bool,
    completed: u32,
}

impl<B: Binder + Sync> Executor<B> {
    pub fn run(&self, resolved: &ResolvedJob) -> Result<ExecOutcome, ExecFailure> {
        let progress = match &self.progress_snapshot {
            Some(path) => Progress::with_snapshot(self.progress, path.clone()),
            None => Progress::new(self.progress),
        };
        let total = resolved.steps.len() as u32;
        let audio = resolved.job.input.audio.as_deref();
        let model = status::engine_from_steps(&resolved.steps);
        status::emit_start(&progress, audio, model.as_deref());

        let wall_start = SystemTime::now();
        let started = Instant::now();
        let max_parallel = resolved.job.max_parallel.unwrap_or(1).max(1);
        let continue_on_error = resolved.job.continue_on_error;
        let pool = Arc::new(ResourcePool::new(resolve_job_capacity(&resolved.job)));

        let mut state = RunState {
            artifacts: ArtifactRegistry::new(),
            prev: None,
            last_out: None,
            active_timemap: None,
            step_reports: Vec::new(),
            any_failed: false,
            completed: 0,
        };

        if let Err(e) = self.run_plan(
            &resolved.plan,
            resolved,
            &progress,
            total,
            max_parallel,
            continue_on_error,
            &pool,
            &mut state,
        ) {
            let report = finish_report(resolved, wall_start, started, state.step_reports, true);
            return Err(ExecFailure { error: e, report });
        }

        let report = finish_report(
            resolved,
            wall_start,
            started,
            state.step_reports,
            state.any_failed,
        );
        let out = match state.last_out {
            Some(p) => p,
            None => {
                let err = ExecError::Other("job produced no output".into());
                return Err(ExecFailure { error: err, report });
            }
        };
        status::emit_done(&progress, Some(&out), started.elapsed().as_secs_f64());
        Ok(ExecOutcome { output: out, report })
    }

    #[allow(clippy::too_many_arguments)]
    fn run_plan(
        &self,
        plan: &WorkflowPlan,
        resolved: &ResolvedJob,
        progress: &Progress,
        total: u32,
        max_parallel: u32,
        continue_on_error: bool,
        pool: &Arc<ResourcePool>,
        state: &mut RunState,
    ) -> Result<(), ExecError> {
        match plan {
            WorkflowPlan::Leaf(idx) => {
                self.run_leaf(*idx, resolved, progress, total, continue_on_error, pool, state)
            }
            WorkflowPlan::Sequence(kids) => {
                for kid in kids {
                    self.run_plan(
                        kid,
                        resolved,
                        progress,
                        total,
                        max_parallel,
                        continue_on_error,
                        pool,
                        state,
                    )?;
                }
                Ok(())
            }
            WorkflowPlan::Parallel(kids) => self.run_parallel(
                kids,
                resolved,
                total,
                max_parallel,
                continue_on_error,
                pool,
                state,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn run_parallel(
        &self,
        kids: &[WorkflowPlan],
        resolved: &ResolvedJob,
        total: u32,
        max_parallel: u32,
        continue_on_error: bool,
        pool: &Arc<ResourcePool>,
        state: &mut RunState,
    ) -> Result<(), ExecError> {
        if kids.is_empty() {
            return Ok(());
        }

        let parent_artifacts = state.artifacts.clone();
        let parent_prev = state.prev.clone();
        let parent_timemap = state.active_timemap.clone();
        let reports = Arc::new(Mutex::new(Vec::<StepReport>::new()));
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        let branch_regs = Arc::new(Mutex::new(Vec::<ArtifactRegistry>::new()));
        let branch_last = Arc::new(Mutex::new(Vec::<Option<PathBuf>>::new()));
        let any_failed = Arc::new(Mutex::new(false));

        for chunk in kids.chunks(max_parallel as usize) {
            thread::scope(|scope| {
                for kid in chunk {
                    let parent_artifacts = parent_artifacts.clone();
                    let parent_prev = parent_prev.clone();
                    let parent_timemap = parent_timemap.clone();
                    let reports = Arc::clone(&reports);
                    let errors = Arc::clone(&errors);
                    let branch_regs = Arc::clone(&branch_regs);
                    let branch_last = Arc::clone(&branch_last);
                    let any_failed = Arc::clone(&any_failed);
                    let pool = Arc::clone(pool);
                    scope.spawn(move || {
                        let quiet = Progress::new(ProgressMode::None);
                        let mut branch = RunState {
                            artifacts: parent_artifacts,
                            prev: parent_prev,
                            last_out: None,
                            active_timemap: parent_timemap,
                            step_reports: Vec::new(),
                            any_failed: false,
                            completed: 0,
                        };
                        let r = self.run_plan(
                            kid,
                            resolved,
                            &quiet,
                            total,
                            max_parallel,
                            continue_on_error,
                            &pool,
                            &mut branch,
                        );
                        reports.lock().unwrap().extend(branch.step_reports);
                        branch_regs.lock().unwrap().push(branch.artifacts);
                        branch_last.lock().unwrap().push(branch.last_out);
                        if branch.any_failed {
                            *any_failed.lock().unwrap() = true;
                        }
                        if let Err(e) = r {
                            errors.lock().unwrap().push(e.to_string());
                        }
                    });
                }
            });
        }

        for reg in branch_regs.lock().unwrap().drain(..) {
            state.artifacts.merge(reg);
        }
        for rep in reports.lock().unwrap().drain(..) {
            state.step_reports.push(rep);
            state.completed += 1;
        }
        for last in branch_last.lock().unwrap().drain(..) {
            if let Some(p) = last {
                state.prev = Some(p.clone());
                state.last_out = Some(p);
            }
        }

        if *any_failed.lock().unwrap() {
            state.any_failed = true;
        }
        let errs = errors.lock().unwrap();
        if let Some(msg) = errs.first() {
            state.any_failed = true;
            if !continue_on_error {
                return Err(ExecError::Step(msg.clone()));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn run_leaf(
        &self,
        idx: usize,
        resolved: &ResolvedJob,
        progress: &Progress,
        total: u32,
        continue_on_error: bool,
        pool: &Arc<ResourcePool>,
        state: &mut RunState,
    ) -> Result<(), ExecError> {
        let step = &resolved.steps[idx];
        let job_step = resolve::leaf_step_at(&resolved.job, idx)
            .ok_or_else(|| ExecError::Other(format!("missing leaf step {idx}")))?;
        let overall = status::overall_percent(state.completed, total);

        if step.skip {
            let now = SystemTime::now();
            status::emit_step_skipped(progress, step, total, overall);
            state.step_reports.push(make_step_report(
                step,
                StepReportStatus::Skipped,
                now,
                now,
                std::time::Duration::ZERO,
                None,
                &[],
            ));
            state.completed += 1;
            return Ok(());
        }

        let artifacts_map = state.artifacts.paths_map();
        let input = match resolve::exec_input(
            job_step,
            &resolved.job,
            &resolved.working_dir,
            &artifacts_map,
            state.prev.as_ref(),
        ) {
            Ok(p) => p,
            Err(e) => {
                let err = ExecError::Step(e.to_string());
                status::emit_error(progress, "step_failed", &err.to_string());
                let now = SystemTime::now();
                state.step_reports.push(make_step_report(
                    step,
                    StepReportStatus::Failed,
                    now,
                    now,
                    std::time::Duration::ZERO,
                    None,
                    &[],
                ));
                state.any_failed = true;
                return Err(err);
            }
        };

        let mut live = step.clone();
        live.input = Some(input.clone());
        status::emit_step_start(progress, &live, total, overall);

        let mut options = step.options.clone();
        if step.capability == Capability::Postprocess {
            if let Err(e) =
                resolve_postprocess_inputs(&mut options, &artifacts_map, &resolved.working_dir)
            {
                let err = ExecError::Step(e);
                status::emit_error(progress, "step_failed", &err.to_string());
                let now = SystemTime::now();
                state.step_reports.push(make_step_report(
                    step,
                    StepReportStatus::Failed,
                    now,
                    now,
                    std::time::Duration::ZERO,
                    Some(&input),
                    &[],
                ));
                state.any_failed = true;
                return Err(err);
            }
        }
        if step.capability == Capability::MeetingMerge {
            if let Err(e) =
                resolve_meeting_merge_inputs(&mut options, job_step, &artifacts_map, &resolved.working_dir)
            {
                let err = ExecError::Step(e);
                status::emit_error(progress, "step_failed", &err.to_string());
                let now = SystemTime::now();
                state.step_reports.push(make_step_report(
                    step,
                    StepReportStatus::Failed,
                    now,
                    now,
                    std::time::Duration::ZERO,
                    Some(&input),
                    &[],
                ));
                state.any_failed = true;
                return Err(err);
            }
        }
        if step.capability == Capability::FixLayout
            && options.get("use_timemap").and_then(ArgValue::as_bool) != Some(false)
        {
            if let Some(tm) = &state.active_timemap {
                options
                    .entry("timemap".into())
                    .or_insert_with(|| ArgValue::String(tm.display().to_string()));
            }
        }

        let req = InvokeRequest {
            capability: step.capability,
            step_id: step.id.clone(),
            working_dir: resolved.working_dir.clone(),
            input: input.clone(),
            output: step.output.clone(),
            output_dir: resolved.job.output.dir.clone(),
            context_assets: resolved.job.context.assets.clone(),
            options,
            progress_snapshot: self.progress_snapshot.clone(),
            progress_step_base: Some(overall),
            progress_step_span: Some(if total == 0 {
                100
            } else {
                (100 / total).max(1) as u8
            }),
        };

        // Contended classes (metal_gpu, …) serialize even when parallel branches fan out.
        let need = step_resource_need(step, job_step);
        let _lease = pool.acquire(&need);

        let queued_at = SystemTime::now();
        let step_t0 = Instant::now();
        match self.binder.invoke(&req) {
            Ok(result) => {
                let step_elapsed = step_t0.elapsed();
                let step_end = SystemTime::now();

                if step.capability == Capability::Preprocess {
                    if let Some(tm) = result.outputs.get("timemap") {
                        state.active_timemap = Some(tm.clone());
                    }
                }

                // ADR §6: remap timeline artifacts from processed → original clock.
                if matches!(
                    step.capability,
                    Capability::Transcribe | Capability::Diarize
                ) {
                    if let Some(tm_path) = state.active_timemap.clone() {
                        let map = timemap::load_timemap(&tm_path)?;
                        timemap::remap_timeline_outputs(
                            &result.primary_output,
                            &result.outputs,
                            &map,
                        )?;
                    }
                }

                state.artifacts.publish_step(
                    step.capability,
                    step.id.as_deref(),
                    &step.produces,
                    &result.primary_output,
                    &result.outputs,
                );
                for (name, path) in &step.outputs {
                    state.artifacts.insert(
                        name.clone(),
                        step.capability.default_artifact_kind(),
                        path.clone(),
                    );
                }
                let mut outs = vec![result.primary_output.clone()];
                outs.extend(result.outputs.values().cloned());
                state.step_reports.push(make_step_report(
                    step,
                    StepReportStatus::Ok,
                    queued_at,
                    step_end,
                    step_elapsed,
                    Some(&input),
                    &outs,
                ));
                state.prev = Some(result.primary_output.clone());
                state.last_out = Some(result.primary_output.clone());
                state.completed += 1;
                let overall_done = status::overall_percent(state.completed, total);
                status::emit_step_done(
                    progress,
                    &live,
                    total,
                    overall_done,
                    &result.primary_output,
                );
                Ok(())
            }
            Err(e) => {
                let step_elapsed = step_t0.elapsed();
                let step_end = SystemTime::now();
                status::emit_error(progress, "step_failed", &e.to_string());
                state.step_reports.push(make_step_report(
                    step,
                    StepReportStatus::Failed,
                    queued_at,
                    step_end,
                    step_elapsed,
                    Some(&input),
                    &[],
                ));
                state.any_failed = true;
                state.completed += 1;
                if continue_on_error {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}

fn finish_report(
    resolved: &ResolvedJob,
    wall_start: SystemTime,
    started: Instant,
    steps: Vec<StepReport>,
    failed: bool,
) -> ExecutionReport {
    let wall_end = SystemTime::now();
    let duration = duration_ms(started.elapsed());
    let critical_path = steps.iter().map(|s| s.duration_ms).max().unwrap_or(0);
    let work_sum: u64 = steps.iter().map(|s| s.duration_ms).sum();
    let parallel_efficiency = if duration == 0 {
        1.0
    } else {
        (work_sum as f64 / duration as f64).min(1.0)
    };

    ExecutionReport {
        version: report::REPORT_VERSION,
        job: resolved.job.name.clone(),
        status: if failed {
            JobReportStatus::Failed
        } else {
            JobReportStatus::Ok
        },
        started_at: format_rfc3339(wall_start),
        finished_at: format_rfc3339(wall_end),
        duration_ms: duration,
        critical_path_ms: Some(critical_path),
        parallel_efficiency: Some(parallel_efficiency),
        steps,
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
    if let Some(p) = resolved.job.max_parallel {
        lines.push(format!("max_parallel: {p}"));
    }
    lines.push(format!("leaves: {}", resolved.steps.len()));
    lines.push("workflow:".into());
    push_plan_text(&mut lines, &resolved.plan, resolved, 1);
    lines.join("\n")
}

fn push_plan_text(
    lines: &mut Vec<String>,
    plan: &WorkflowPlan,
    resolved: &ResolvedJob,
    depth: usize,
) {
    let pad = "  ".repeat(depth);
    match plan {
        WorkflowPlan::Leaf(i) => {
            let s = &resolved.steps[*i];
            let mut parts = vec![format!("{}. {} [{}]", s.index, s.capability.as_str(), s.path)];
            if s.skip {
                parts.push("skip".into());
            }
            if let Some(id) = &s.id {
                parts.push(format!("id={id}"));
            }
            if !s.produces.is_empty() {
                parts.push(format!("produces=[{}]", s.produces.join(", ")));
            }
            if !s.consumes.is_empty() {
                parts.push(format!("consumes=[{}]", s.consumes.join(", ")));
            }
            if let Some(engine) = s.options.get("engine").and_then(ArgValue::as_string) {
                parts.push(format!("engine={engine}"));
            }
            if let Some(model) = s.options.get("model").and_then(ArgValue::as_string) {
                parts.push(format!("model={model}"));
            }
            lines.push(format!("{pad}{}", parts.join("  ")));
        }
        WorkflowPlan::Sequence(kids) => {
            lines.push(format!("{pad}sequence:"));
            for k in kids {
                push_plan_text(lines, k, resolved, depth + 1);
            }
        }
        WorkflowPlan::Parallel(kids) => {
            lines.push(format!("{pad}parallel:"));
            for k in kids {
                push_plan_text(lines, k, resolved, depth + 1);
            }
        }
    }
}

pub fn dry_run_json(resolved: &ResolvedJob) -> Result<String, ExecError> {
    serde_json::to_string_pretty(&resolved.job).map_err(|e| ExecError::Other(e.to_string()))
}

fn resolve_postprocess_inputs(
    options: &mut BTreeMap<String, ArgValue>,
    artifacts: &HashMap<String, PathBuf>,
    working_dir: &Path,
) -> Result<(), String> {
    let Some(ArgValue::Map(map)) = options.get("inputs").cloned() else {
        return Ok(());
    };
    let mut resolved = BTreeMap::new();
    for (name, v) in map {
        let Some(raw) = v.as_string() else {
            continue;
        };
        let path = match ArtifactRef::parse(&raw) {
            ArtifactRef::Id(id) if id.ends_with("/*") => {
                let prefix = id.trim_end_matches("/*");
                artifacts
                    .iter()
                    .find(|(k, _)| k.starts_with(prefix))
                    .map(|(_, p)| p.clone())
                    .ok_or_else(|| format!("postprocess input '{name}': no match for {id}"))?
            }
            ArtifactRef::Id(id) => artifacts
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("postprocess input '{name}': artifact not produced: {id}"))?,
            ArtifactRef::Path(p) => {
                if p.is_absolute() {
                    p
                } else {
                    working_dir.join(p)
                }
            }
        };
        resolved.insert(name, ArgValue::String(path.display().to_string()));
    }
    options.insert("inputs".into(), ArgValue::Map(resolved));
    Ok(())
}

fn resolve_meeting_merge_inputs(
    options: &mut BTreeMap<String, ArgValue>,
    step: &crate::job::Step,
    artifacts: &HashMap<String, PathBuf>,
    working_dir: &Path,
) -> Result<(), String> {
    let mut resolved = BTreeMap::new();
    for raw in step.input_refs() {
        let path = match ArtifactRef::parse(raw) {
            ArtifactRef::Id(id) => artifacts
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("meeting-merge input artifact not produced: {id}"))?,
            ArtifactRef::Path(p) => {
                if p.is_absolute() {
                    p
                } else {
                    working_dir.join(p)
                }
            }
        };
        resolved.insert(raw.to_string(), ArgValue::String(path.display().to_string()));
    }
    // Also resolve named option refs (texts / mix / timeline) when they are artifact ids.
    for key in ["mix", "timeline"] {
        if let Some(raw) = options.get(key).and_then(ArgValue::as_string) {
            if let ArtifactRef::Id(id) = ArtifactRef::parse(&raw) {
                if let Some(p) = artifacts.get(&id) {
                    options.insert(key.into(), ArgValue::String(p.display().to_string()));
                }
            } else if let ArtifactRef::Path(p) = ArtifactRef::parse(&raw) {
                let path = if p.is_absolute() {
                    p
                } else {
                    working_dir.join(p)
                };
                options.insert(key.into(), ArgValue::String(path.display().to_string()));
            }
        }
    }
    if let Some(ArgValue::Strings(ids)) = options.get("texts").cloned() {
        let mut text_paths = BTreeMap::new();
        for id in ids {
            let path = resolved
                .get(&id)
                .and_then(ArgValue::as_string)
                .map(PathBuf::from)
                .or_else(|| artifacts.get(&id).cloned())
                .ok_or_else(|| format!("meeting-merge text artifact missing: {id}"))?;
            text_paths.insert(id, ArgValue::String(path.display().to_string()));
        }
        options.insert("text_paths".into(), ArgValue::Map(text_paths));
    }
    options.insert("resolved_inputs".into(), ArgValue::Map(resolved));
    Ok(())
}
