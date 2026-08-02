//! Resolve working_dir, validate artifact refs / workflow, gate engines.

use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::path::{Path, PathBuf};

use super::schema::{
    ArgValue, ArtifactRef, Capability, Job, JobError, ResolvedJob, ResolvedStep, Step,
    TranscribeEngine, WorkflowNode, WorkflowPlan,
};

pub fn resolve_job(job: Job) -> Result<ResolvedJob, JobError> {
    let working_dir = match &job.working_dir {
        Some(p) if p.as_os_str().is_empty() => cwd()?,
        Some(p) => absolutize(p)?,
        None => cwd()?,
    };

    if job.leaf_count() == 0 {
        return Err(JobError::Usage("job has no capability steps".into()));
    }

    gate_engines(&job)?;
    gate_capabilities(&job)?;
    validate_artifact_refs(&job)?;

    let mut leaves = Vec::new();
    let mut leaf_index = 0usize;
    let plan = compile_plan(&job.steps, "", &mut leaves, &mut leaf_index, &job, &working_dir)?;

    let order = schedule_leaf_order(&job, leaves.len())?;

    Ok(ResolvedJob {
        job,
        working_dir,
        steps: leaves,
        plan,
        order,
    })
}

fn compile_plan(
    nodes: &[WorkflowNode],
    prefix: &str,
    leaves: &mut Vec<ResolvedStep>,
    leaf_index: &mut usize,
    job: &Job,
    working_dir: &Path,
) -> Result<WorkflowPlan, JobError> {
    // Root / sequence list: if every node is a Step, still wrap as Sequence.
    let mut kids = Vec::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        let path = if prefix.is_empty() {
            format!("{i}")
        } else {
            format!("{prefix}.{i}")
        };
        kids.push(compile_node(node, &path, leaves, leaf_index, job, working_dir)?);
    }
    Ok(WorkflowPlan::Sequence(kids))
}

fn compile_node(
    node: &WorkflowNode,
    path: &str,
    leaves: &mut Vec<ResolvedStep>,
    leaf_index: &mut usize,
    job: &Job,
    working_dir: &Path,
) -> Result<WorkflowPlan, JobError> {
    match node {
        WorkflowNode::Step(step) => {
            let idx = *leaf_index;
            *leaf_index += 1;
            let input = if step.skip {
                None
            } else {
                preview_input(step, job, working_dir)?
            };
            let output = step
                .output
                .as_ref()
                .map(|p| resolve_against(working_dir, p));
            let outputs = step
                .outputs
                .iter()
                .map(|(k, p)| (k.clone(), resolve_against(working_dir, p)))
                .collect();
            leaves.push(ResolvedStep {
                index: (idx + 1) as u32,
                path: path.to_string(),
                capability: step.r#use,
                id: step.id.clone(),
                name: step.name.clone(),
                skip: step.skip,
                input,
                output,
                outputs,
                produces: step.produces.clone(),
                consumes: step.consumes.clone(),
                options: step.options.clone(),
            });
            Ok(WorkflowPlan::Leaf(idx))
        }
        WorkflowNode::Sequence { sequence, .. } => {
            compile_plan(sequence, path, leaves, leaf_index, job, working_dir)
        }
        WorkflowNode::Parallel { parallel, .. } => {
            let mut kids = Vec::with_capacity(parallel.len());
            for (i, child) in parallel.iter().enumerate() {
                let child_path = format!("{path}.{i}");
                kids.push(compile_node(
                    child,
                    &child_path,
                    leaves,
                    leaf_index,
                    job,
                    working_dir,
                )?);
            }
            Ok(WorkflowPlan::Parallel(kids))
        }
    }
}

fn preview_input(
    step: &Step,
    job: &Job,
    working_dir: &Path,
) -> Result<Option<PathBuf>, JobError> {
    let refs = step.input_refs();
    if let Some(raw) = refs.first() {
        return match ArtifactRef::parse(raw) {
            ArtifactRef::Id(_) => Ok(None),
            ArtifactRef::Path(p) => Ok(Some(resolve_against(working_dir, p.as_path()))),
        };
    }
    match step.r#use {
        Capability::Transcribe | Capability::Diarize | Capability::Preprocess => {
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
        | Capability::FixLayout
        | Capability::MeetingMerge
        | Capability::Postprocess => Ok(None),
    }
}

fn validate_artifact_refs(job: &Job) -> Result<(), JobError> {
    let leaves: Vec<&Step> = job.leaf_steps();
    let mut produced: HashSet<String> = HashSet::new();
    let mut step_ids: HashSet<String> = HashSet::new();

    for step in &leaves {
        if let Some(id) = &step.id {
            if !step_ids.insert(id.clone()) {
                return Err(JobError::Usage(format!("duplicate step id: {id}")));
            }
            if !produced.insert(id.clone()) {
                return Err(JobError::Usage(format!("duplicate artifact id: {id}")));
            }
        }
        for name in step.outputs.keys() {
            if !produced.insert(name.clone()) {
                return Err(JobError::Usage(format!("duplicate artifact id: {name}")));
            }
        }
        for name in &step.produces {
            if name.contains('*') {
                continue;
            }
            produced.insert(name.clone());
        }
    }

    for step in &leaves {
        for raw in step.input_refs() {
            match ArtifactRef::parse(raw) {
                ArtifactRef::Path(_) => {}
                ArtifactRef::Id(id) if id.contains('*') => {
                    // Wildcard: prefix must match at least a naming convention; checked at runtime.
                }
                ArtifactRef::Id(id) => {
                    if !produced.contains(&id) && !step_ids.contains(&id) {
                        return Err(JobError::Usage(format!(
                            "unknown artifact id in inputs: {id}"
                        )));
                    }
                }
            }
        }
        for dep in &step.depends {
            if !step_ids.contains(dep) {
                return Err(JobError::Usage(format!("unknown depends id: {dep}")));
            }
        }
        if step.r#use == Capability::Postprocess {
            if let Some(map) = step.options.get("inputs").and_then(ArgValue::as_map) {
                for v in map.values() {
                    if let Some(raw) = v.as_string() {
                        if let ArtifactRef::Id(id) = ArtifactRef::parse(&raw) {
                            if !id.contains('*') && !produced.contains(&id) && !step_ids.contains(&id)
                            {
                                return Err(JobError::Usage(format!(
                                    "unknown artifact id in postprocess inputs: {id}"
                                )));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Kahn topo over **leaf** indices (for validation + flat-sequence scheduling hints).
fn schedule_leaf_order(job: &Job, n: usize) -> Result<Vec<usize>, JobError> {
    let leaves: Vec<&Step> = job.leaf_steps();
    debug_assert_eq!(leaves.len(), n);

    let mut id_to_idx: HashMap<&str, usize> = HashMap::new();
    let mut artifact_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, step) in leaves.iter().enumerate() {
        if let Some(id) = &step.id {
            id_to_idx.insert(id.as_str(), i);
            artifact_to_idx.insert(id.as_str(), i);
        }
        for name in step.outputs.keys() {
            artifact_to_idx.insert(name.as_str(), i);
        }
        for name in &step.produces {
            if !name.contains('*') {
                artifact_to_idx.insert(name.as_str(), i);
            }
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

    for (i, step) in leaves.iter().enumerate() {
        for raw in step.input_refs() {
            if let ArtifactRef::Id(id) = ArtifactRef::parse(raw) {
                if id.contains('*') {
                    continue;
                }
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
                            if id.contains('*') {
                                continue;
                            }
                            if let Some(&from) = artifact_to_idx.get(id.as_str()) {
                                add_edge(&mut adj, &mut indeg, from, i);
                            }
                        }
                    }
                }
            }
        }
        if step.input_refs().is_empty()
            && matches!(
                step.r#use,
                Capability::FixCasing
                    | Capability::FixAsr
                    | Capability::FixTerms
                    | Capability::FixLayout
                    | Capability::MeetingMerge
            )
            && !step.skip
        {
            if let Some(prev) = (0..i).rev().find(|&j| !leaves[j].skip) {
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
    for step in job.leaf_steps() {
        if step.r#use != Capability::Transcribe {
            continue;
        }
        let engine = step
            .options
            .get("engine")
            .and_then(ArgValue::as_string)
            .unwrap_or_else(|| "gigaam".into());
        match TranscribeEngine::parse(&engine) {
            Some(TranscribeEngine::Whisper) => {
                return Err(JobError::Reserved(
                    "whisper is reserved; vd-whisper is not available yet".into(),
                ));
            }
            Some(TranscribeEngine::Gigaam) => {}
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
    for step in job.leaf_steps() {
        if step.r#use.is_reserved() {
            return Err(JobError::Reserved(format!(
                "capability '{}' is reserved",
                step.r#use.as_str()
            )));
        }
        if step.r#use == Capability::Postprocess {
            let has = step
                .options
                .get("recipes")
                .and_then(|v| match v {
                    ArgValue::Strings(s) => Some(!s.is_empty()),
                    ArgValue::String(s) => Some(!s.is_empty()),
                    _ => None,
                })
                .unwrap_or(false);
            if !has {
                return Err(JobError::Usage(
                    "postprocess requires options.recipes".into(),
                ));
            }
        }
        if step.r#use == Capability::Preprocess {
            let has_filters = step
                .options
                .get("filters")
                .and_then(ArgValue::as_list)
                .is_some_and(|l| !l.is_empty());
            let has_chain = step
                .options
                .get("chain")
                .and_then(ArgValue::as_string)
                .is_some_and(|s| !s.is_empty());
            if !has_filters && !has_chain {
                return Err(JobError::Usage(
                    "preprocess requires options.filters or options.chain".into(),
                ));
            }
        }
    }
    Ok(())
}

pub fn exec_input(
    step: &Step,
    job: &Job,
    working_dir: &Path,
    artifacts: &HashMap<String, PathBuf>,
    prev: Option<&PathBuf>,
) -> Result<PathBuf, JobError> {
    let refs = step.input_refs();
    if let Some(raw) = refs.first() {
        return match ArtifactRef::parse(raw) {
            ArtifactRef::Id(id) if id.ends_with("/*") => {
                let prefix = id.trim_end_matches("/*");
                artifacts
                    .iter()
                    .find(|(k, _)| k.starts_with(prefix))
                    .map(|(_, p)| p.clone())
                    .ok_or_else(|| JobError::Usage(format!("no artifact matches wildcard: {id}")))
            }
            ArtifactRef::Id(id) => artifacts.get(&id).cloned().ok_or_else(|| {
                JobError::Usage(format!("artifact not produced yet: {id}"))
            }),
            ArtifactRef::Path(p) => Ok(resolve_against(working_dir, &p)),
        };
    }
    match step.r#use {
        Capability::Transcribe | Capability::Diarize | Capability::Preprocess => {
            let audio = job
                .input
                .audio
                .as_ref()
                .ok_or_else(|| JobError::Usage("missing input.audio".into()))?;
            Ok(resolve_against(working_dir, audio))
        }
        Capability::PrepareContext => {
            let docs = job
                .context
                .docs
                .as_ref()
                .ok_or_else(|| JobError::Usage("missing context.docs".into()))?;
            Ok(resolve_against(working_dir, docs))
        }
        Capability::FixCasing
        | Capability::FixAsr
        | Capability::FixTerms
        | Capability::FixLayout
        | Capability::MeetingMerge
        | Capability::Postprocess => prev.cloned().ok_or_else(|| {
            JobError::Usage(format!(
                "{} needs inputs or a previous step output",
                step.r#use.as_str()
            ))
        }),
    }
}

fn resolve_against(working_dir: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        working_dir.join(p)
    }
}

fn absolutize(p: &Path) -> Result<PathBuf, JobError> {
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(cwd()?.join(p))
    }
}

fn cwd() -> Result<PathBuf, JobError> {
    env::current_dir().map_err(|e| JobError::Other(e.to_string()))
}

/// Look up leaf [`Step`] by resolved leaf index.
pub fn leaf_step_at<'a>(job: &'a Job, leaf_idx: usize) -> Option<&'a Step> {
    job.leaf_steps().into_iter().nth(leaf_idx)
}
