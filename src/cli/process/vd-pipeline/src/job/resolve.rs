//! Resolve working_dir, validate artifact refs, gate engines.

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use super::schema::{
    ArtifactRef, Capability, Job, JobError, ResolvedJob, ResolvedStep, TranscribeEngine,
};

pub fn resolve_job(job: Job) -> Result<ResolvedJob, JobError> {
    let working_dir = match &job.working_dir {
        Some(p) if p.as_os_str().is_empty() => cwd()?,
        Some(p) => absolutize(p)?,
        None => cwd()?,
    };

    gate_engines(&job)?;
    validate_artifact_refs(&job)?;

    let mut resolved_steps = Vec::with_capacity(job.steps.len());
    for (i, step) in job.steps.iter().enumerate() {
        let index = (i + 1) as u32;
        let input = if step.skip {
            None
        } else {
            preview_input(step, &job, &working_dir)?
        };
        let output = step
            .output
            .as_ref()
            .map(|p| resolve_against(&working_dir, p));

        resolved_steps.push(ResolvedStep {
            index,
            capability: step.r#use,
            id: step.id.clone(),
            name: step.name.clone(),
            skip: step.skip,
            input,
            output,
            options: step.options.clone(),
        });
    }

    Ok(ResolvedJob {
        job,
        working_dir,
        steps: resolved_steps,
    })
}

fn preview_input(
    step: &super::schema::Step,
    job: &Job,
    working_dir: &Path,
) -> Result<Option<PathBuf>, JobError> {
    if let Some(raw) = &step.input {
        return match ArtifactRef::parse(raw) {
            ArtifactRef::Id(_) => Ok(None), // filled at exec from artifact map
            ArtifactRef::Path(p) => Ok(Some(resolve_against(working_dir, &p))),
        };
    }
    match step.r#use {
        Capability::Transcribe => {
            let audio = job.input.audio.as_ref().ok_or_else(|| {
                JobError::Usage("transcribe step needs input.audio or step.input".into())
            })?;
            Ok(Some(resolve_against(working_dir, audio)))
        }
        Capability::PrepareContext => {
            let docs = job.context.docs.as_ref().ok_or_else(|| {
                JobError::Usage("prepare-context needs context.docs or step.input".into())
            })?;
            Ok(Some(resolve_against(working_dir, docs)))
        }
        Capability::FixCasing | Capability::FixAsr | Capability::FixTerms => Ok(None),
    }
}

fn validate_artifact_refs(job: &Job) -> Result<(), JobError> {
    let mut available = HashSet::new();
    for step in &job.steps {
        if let Some(raw) = &step.input {
            if let ArtifactRef::Id(id) = ArtifactRef::parse(raw) {
                if !available.contains(&id) {
                    return Err(JobError::Usage(format!(
                        "unknown artifact id in input: {id}"
                    )));
                }
            }
        }
        if let Some(id) = &step.id {
            if !available.insert(id.clone()) {
                return Err(JobError::Usage(format!("duplicate artifact id: {id}")));
            }
        }
    }
    Ok(())
}

fn gate_engines(job: &Job) -> Result<(), JobError> {
    for step in &job.steps {
        if step.r#use != Capability::Transcribe || step.skip {
            continue;
        }
        let engine = step
            .options
            .get("engine")
            .and_then(super::schema::ArgValue::as_string)
            .unwrap_or_else(|| "gigaam".into());
        match TranscribeEngine::parse(&engine) {
            Some(TranscribeEngine::Gigaam) => {}
            Some(TranscribeEngine::Whisper) => {
                return Err(JobError::Reserved(
                    "whisper is reserved; vd-whisper is not available yet".into(),
                ));
            }
            None => {
                return Err(JobError::Usage(format!(
                    "unknown transcribe engine: {engine}"
                )));
            }
        }
    }
    Ok(())
}

fn resolve_against(base: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

fn absolutize(p: &Path) -> Result<PathBuf, JobError> {
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    Ok(cwd()?.join(p))
}

fn cwd() -> Result<PathBuf, JobError> {
    env::current_dir().map_err(|e| JobError::Other(format!("current_dir: {e}")))
}

/// Resolve step input at execution time.
pub fn exec_input(
    step: &super::schema::Step,
    job: &Job,
    working_dir: &Path,
    artifacts: &std::collections::HashMap<String, PathBuf>,
    prev: Option<&PathBuf>,
) -> Result<PathBuf, JobError> {
    if let Some(raw) = &step.input {
        return match ArtifactRef::parse(raw) {
            ArtifactRef::Id(id) => artifacts
                .get(&id)
                .cloned()
                .ok_or_else(|| JobError::Usage(format!("artifact not produced yet: {id}"))),
            ArtifactRef::Path(p) => Ok(resolve_against(working_dir, &p)),
        };
    }
    match step.r#use {
        Capability::Transcribe => {
            let audio = job.input.audio.as_ref().ok_or_else(|| {
                JobError::Usage("transcribe step needs input.audio or step.input".into())
            })?;
            Ok(resolve_against(working_dir, audio))
        }
        Capability::PrepareContext => {
            let docs = job.context.docs.as_ref().ok_or_else(|| {
                JobError::Usage("prepare-context needs context.docs or step.input".into())
            })?;
            Ok(resolve_against(working_dir, docs))
        }
        Capability::FixCasing | Capability::FixAsr | Capability::FixTerms => {
            prev.cloned().ok_or_else(|| {
                JobError::Usage(format!(
                    "{} step needs input or a previous step output",
                    step.r#use.as_str()
                ))
            })
        }
    }
}
