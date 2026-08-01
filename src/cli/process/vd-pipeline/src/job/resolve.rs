//! Resolve working_dir, validate artifact refs / DAG, gate engines.

use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::path::{Path, PathBuf};

use super::schema::{
    ArgValue, ArtifactRef, Capability, Job, JobError, ResolvedJob, ResolvedStep, TranscribeEngine,
};

pub fn resolve_job(job: Job) -> Result<ResolvedJob, JobError> {
    let working_dir = match &job.working_dir {
        Some(p) if p.as_os_str().is_empty() => cwd()?,
        Some(p) => absolutize(p)?,
        None => cwd()?,
    };

    gate_engines(&job)?;
    gate_capabilities(&job)?;
    validate_artifact_refs(&job)?;
    let order = schedule_order(&job)?;

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
        let outputs = step
            .outputs
            .iter()
            .map(|(k, p)| (k.clone(), resolve_against(&working_dir, p)))
            .collect();

        resolved_steps.push(ResolvedStep {
            index,
            capability: step.r#use,
            id: step.id.clone(),
            name: step.name.clone(),
            skip: step.skip,
            input,
            output,
            outputs,
            options: step.options.clone(),
        });
    }

    Ok(ResolvedJob {
        job,
        working_dir,
        steps: resolved_steps,
        order,
    })
}

fn preview_input(
    step: &super::schema::Step,
    job: &Job,
    working_dir: &Path,
) -> Result<Option<PathBuf>, JobError> {
    let refs = step.input_refs();
    if let Some(raw) = refs.first() {
        return match ArtifactRef::parse(raw) {
            ArtifactRef::Id(_) => Ok(None),
            ArtifactRef::Path(p) => Ok(Some(resolve_against(working_dir, &p))),
        };
    }
    match step.r#use {
        Capability::Transcribe | Capability::Diarize => {
            let audio = job.input.audio.as_ref().ok_or_else(|| {
                JobError::Usage(format!(
                    "{} step needs input.audio or step.inputs",
                    step.r#use.as_str()
                ))
            })?;
            Ok(Some(resolve_against(working_dir, audio)))
        }
        Capability::PrepareContext => {
            let docs = job.context.docs.as_ref().ok_or_else(|| {
                JobError::Usage("prepare-context needs context.docs or step.inputs".into())
            })?;
            Ok(Some(resolve_against(working_dir, docs)))
        }
        Capability::FixCasing
        | Capability::FixAsr
        | Capability::FixTerms
        | Capability::MeetingMerge
        | Capability::Postprocess => Ok(None),
    }
}

fn validate_artifact_refs(job: &Job) -> Result<(), JobError> {
    let mut produced: HashSet<String> = HashSet::new();
    let mut step_ids: HashSet<String> = HashSet::new();
    for step in &job.steps {
        if let Some(id) = &step.id {
            if !produced.insert(id.clone()) {
                return Err(JobError::Usage(format!("duplicate artifact id: {id}")));
            }
            if !step_ids.insert(id.clone()) {
                return Err(JobError::Usage(format!("duplicate step id: {id}")));
            }
        }
        for name in step.outputs.keys() {
            if !produced.insert(name.clone()) {
                return Err(JobError::Usage(format!("duplicate artifact id: {name}")));
            }
        }
    }
    for step in &job.steps {
        for raw in step.input_refs() {
            if let ArtifactRef::Id(id) = ArtifactRef::parse(raw) {
                if !produced.contains(&id) {
                    return Err(JobError::Usage(format!(
                        "unknown artifact id in inputs: {id}"
                    )));
                }
            }
        }
        if step.r#use == Capability::Postprocess {
            if let Some(map) = step.options.get("inputs").and_then(ArgValue::as_map) {
                for v in map.values() {
                    if let Some(raw) = v.as_string() {
                        if let ArtifactRef::Id(id) = ArtifactRef::parse(&raw) {
                            if !produced.contains(&id) {
                                return Err(JobError::Usage(format!(
                                    "unknown artifact id in postprocess options.inputs: {id}"
                                )));
                            }
                        }
                    }
                }
            }
        }
        for dep in &step.depends {
            if !step_ids.contains(dep) {
                return Err(JobError::Usage(format!("unknown depends step id: {dep}")));
            }
        }
    }
    Ok(())
}

/// Kahn topo order. Edges: producer → consumer via artifact `inputs` / `depends`.
pub fn schedule_order(job: &Job) -> Result<Vec<usize>, JobError> {
    let n = job.steps.len();
    let mut id_to_idx: HashMap<&str, usize> = HashMap::new();
    let mut artifact_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, step) in job.steps.iter().enumerate() {
        if let Some(id) = &step.id {
            id_to_idx.insert(id.as_str(), i);
            artifact_to_idx.insert(id.as_str(), i);
        }
        for name in step.outputs.keys() {
            artifact_to_idx.insert(name.as_str(), i);
        }
    }

    let mut indeg = vec![0u32; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    let add_edge = |adj: &mut Vec<Vec<usize>>, indeg: &mut [u32], from: usize, to: usize| {
        if from == to {
            return;
        }
        if !adj[from].contains(&to) {
            adj[from].push(to);
            indeg[to] += 1;
        }
    };

    for (i, step) in job.steps.iter().enumerate() {
        for raw in step.input_refs() {
            if let ArtifactRef::Id(id) = ArtifactRef::parse(raw) {
                if let Some(&from) = artifact_to_idx.get(id.as_str()) {
                    add_edge(&mut adj, &mut indeg, from, i);
                }
            }
        }
        for dep in &step.depends {
            if let Some(&from) = id_to_idx.get(dep.as_str()) {
                add_edge(&mut adj, &mut indeg, from, i);
            }
        }
        if step.r#use == Capability::Postprocess {
            if let Some(map) = step.options.get("inputs").and_then(ArgValue::as_map) {
                for v in map.values() {
                    if let Some(raw) = v.as_string() {
                        if let ArtifactRef::Id(id) = ArtifactRef::parse(&raw) {
                            if let Some(&from) = artifact_to_idx.get(id.as_str()) {
                                add_edge(&mut adj, &mut indeg, from, i);
                            }
                        }
                    }
                }
            }
        }
        // Linear sugar: no inputs → depend on previous non-skip step in declaration order
        // only when capability needs a previous primary (fix / merge without refs).
        if step.input_refs().is_empty()
            && matches!(
                step.r#use,
                Capability::FixCasing
                    | Capability::FixAsr
                    | Capability::FixTerms
                    | Capability::MeetingMerge
            )
            && !step.skip
        {
            if let Some(prev) = (0..i).rev().find(|&j| !job.steps[j].skip) {
                add_edge(&mut adj, &mut indeg, prev, i);
            }
        }
    }

    let mut ready: Vec<usize> = indeg
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| (d == 0).then_some(i))
        .collect();
    ready.sort_unstable();
    let mut q: VecDeque<usize> = ready.into();

    let mut order = Vec::with_capacity(n);
    while let Some(i) = q.pop_front() {
        order.push(i);
        let mut nxt: Vec<usize> = Vec::new();
        for &j in &adj[i] {
            indeg[j] -= 1;
            if indeg[j] == 0 {
                nxt.push(j);
            }
        }
        nxt.sort_unstable();
        q.extend(nxt);
    }

    if order.len() != n {
        return Err(JobError::Usage("job DAG has a cycle".into()));
    }
    Ok(order)
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

fn gate_capabilities(job: &Job) -> Result<(), JobError> {
    for step in &job.steps {
        if step.skip {
            continue;
        }
        if step.r#use.is_reserved() {
            return Err(JobError::Reserved(format!(
                "{} is reserved; not available yet",
                step.r#use.as_str()
            )));
        }
        if step.r#use == Capability::Postprocess {
            gate_postprocess_options(step)?;
        }
    }
    Ok(())
}

fn gate_postprocess_options(step: &super::schema::Step) -> Result<(), JobError> {
    let recipes = step.options.get("recipes");
    let empty = match recipes {
        None => true,
        Some(ArgValue::Strings(v)) => v.is_empty(),
        Some(ArgValue::String(s)) => s.is_empty(),
        Some(ArgValue::Map(m)) => m.is_empty(),
        Some(_) => false,
    };
    if empty {
        return Err(JobError::Usage(
            "postprocess step requires options.recipes (non-empty)".into(),
        ));
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

/// Resolve primary step input at execution time.
pub fn exec_input(
    step: &super::schema::Step,
    job: &Job,
    working_dir: &Path,
    artifacts: &HashMap<String, PathBuf>,
    prev: Option<&PathBuf>,
) -> Result<PathBuf, JobError> {
    let refs = step.input_refs();
    if let Some(raw) = refs.first() {
        return match ArtifactRef::parse(raw) {
            ArtifactRef::Id(id) => artifacts
                .get(&id)
                .cloned()
                .ok_or_else(|| JobError::Usage(format!("artifact not produced yet: {id}"))),
            ArtifactRef::Path(p) => Ok(resolve_against(working_dir, &p)),
        };
    }
    match step.r#use {
        Capability::Transcribe | Capability::Diarize => {
            let audio = job.input.audio.as_ref().ok_or_else(|| {
                JobError::Usage(format!(
                    "{} step needs input.audio or step.inputs",
                    step.r#use.as_str()
                ))
            })?;
            Ok(resolve_against(working_dir, audio))
        }
        Capability::PrepareContext => {
            let docs = job.context.docs.as_ref().ok_or_else(|| {
                JobError::Usage("prepare-context needs context.docs or step.inputs".into())
            })?;
            Ok(resolve_against(working_dir, docs))
        }
        Capability::FixCasing
        | Capability::FixAsr
        | Capability::FixTerms
        | Capability::MeetingMerge
        | Capability::Postprocess => prev.cloned().ok_or_else(|| {
            JobError::Usage(format!(
                "{} step needs inputs or a previous step output",
                step.r#use.as_str()
            ))
        }),
    }
}
