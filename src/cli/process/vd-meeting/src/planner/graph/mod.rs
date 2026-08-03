//! Build Job workflow tree from ResolvedMeeting.

mod diarize;
mod merge;
mod preprocess;
mod transcript;

use vd_pipeline::{
    ArgValue, Capability, Job, JobContext, JobInput, JobOutput, Step, WorkflowNode,
};

use super::normalize::ResolvedMeeting;
use super::PlanError;
use crate::model::{AlignmentReference, DiarizationEnabled, InputPurpose, InputRole};
use crate::model::BuildOptions;

pub fn build_job(
    resolved: &ResolvedMeeting,
    options: &BuildOptions,
) -> Result<Job, PlanError> {
    let mut root: Vec<WorkflowNode> = Vec::new();

    if resolved.has_context {
        if let Some(ctx) = resolved.inputs.iter().find(|i| i.role == InputRole::Context) {
            let mut step = Step::new(Capability::PrepareContext);
            step.id = Some("assets".into());
            step.input = Some(ctx.path.display().to_string());
            root.push(step.into());
        }
    }

    let mut branch_nodes: Vec<WorkflowNode> = Vec::new();
    let mut text_ids: Vec<String> = Vec::new();
    for &idx in &resolved.text_sources {
        let src = &resolved.inputs[idx];
        let mut leafs: Vec<Step> = Vec::new();
        let final_id = transcript::append_branch(&mut leafs, src, options)?;
        text_ids.push(final_id);
        let seq: Vec<WorkflowNode> = leafs.into_iter().map(Into::into).collect();
        branch_nodes.push(WorkflowNode::sequence(seq));
    }

    if branch_nodes.len() == 1 {
        // Flatten single-branch sequence into root.
        if let WorkflowNode::Sequence { sequence, .. } = branch_nodes.remove(0) {
            root.extend(sequence);
        }
    } else if branch_nodes.len() > 1 {
        root.push(WorkflowNode::parallel(branch_nodes));
    }

    let want_diarize = should_diarize(resolved)?;
    let timeline_id = if want_diarize {
        let mut leafs = Vec::new();
        let id = diarize::append_diarize(&mut leafs, resolved, options)?;
        root.extend(leafs.into_iter().map(Into::into));
        Some(id)
    } else {
        None
    };

    let mix_ref = if wants_mix_reference(resolved, want_diarize) {
        let mut leafs = Vec::new();
        let id = append_mix_ref(&mut leafs, resolved, options)?;
        root.extend(leafs.into_iter().map(Into::into));
        Some(id)
    } else {
        None
    };

    if text_ids.is_empty() && timeline_id.is_none() && mix_ref.is_none() {
        return Err(PlanError::Usage(
            "planner produced no mergeable artifacts".into(),
        ));
    }

    let mut merge_leafs = Vec::new();
    merge::append_merge(
        &mut merge_leafs,
        resolved,
        &text_ids,
        timeline_id.as_deref(),
        mix_ref.as_deref(),
        options,
    )?;
    root.extend(merge_leafs.into_iter().map(Into::into));

    let context = if resolved.has_context {
        let docs = resolved
            .inputs
            .iter()
            .find(|i| i.role == InputRole::Context)
            .map(|i| i.path.clone());
        JobContext {
            docs,
            assets: Some(resolved.working_dir.join(".voxdecoder")),
        }
    } else {
        JobContext::default()
    };

    let audio = resolved
        .inputs
        .iter()
        .find(|i| i.role == InputRole::Room)
        .or_else(|| {
            resolved
                .inputs
                .iter()
                .find(|i| i.role == InputRole::Participant)
        })
        .and_then(|i| {
            if i.url.is_some() {
                None
            } else if i.path.as_os_str().is_empty() {
                None
            } else {
                Some(i.path.clone())
            }
        });

    Ok(Job {
        version: 1,
        name: Some("meeting".into()),
        working_dir: Some(resolved.working_dir.clone()),
        input: JobInput { audio },
        context,
        output: JobOutput {
            dir: resolved.output.dir.clone(),
        },
        continue_on_error: options.executor.continue_on_error,
        max_parallel: options.executor.max_parallel.or(Some(4)),
        resources: default_meeting_resources(&options.executor.resources),
        steps: root,
    })
}

fn default_meeting_resources(
    explicit: &std::collections::BTreeMap<String, u32>,
) -> std::collections::BTreeMap<String, u32> {
    let mut resources = explicit.clone();
    #[cfg(target_os = "macos")]
    {
        // Parallel transcript branches share one Metal slot (Executor + srv Resource Manager).
        resources.entry("metal_gpu".into()).or_insert(1);
    }
    resources
}

fn should_diarize(resolved: &ResolvedMeeting) -> Result<bool, PlanError> {
    match resolved.meeting.alignment.reference {
        AlignmentReference::None | AlignmentReference::Mix => {
            // Explicit tracks-only or align-to-mix: never run diarize.
            return Ok(false);
        }
        AlignmentReference::Timeline => {
            if resolved.timeline_sources.is_empty() {
                return Err(PlanError::Usage(
                    "alignment.reference=timeline requires a room/timeline input".into(),
                ));
            }
            if matches!(
                resolved.meeting.diarization.enabled,
                DiarizationEnabled::False
            ) {
                return Err(PlanError::Usage(
                    "alignment.reference=timeline requires diarization.enabled != false".into(),
                ));
            }
            return Ok(true);
        }
        AlignmentReference::Auto => {}
    }

    Ok(match resolved.meeting.diarization.enabled {
        DiarizationEnabled::False => false,
        DiarizationEnabled::True | DiarizationEnabled::Auto => {
            !resolved.timeline_sources.is_empty()
        }
    })
}

fn wants_mix_reference(resolved: &ResolvedMeeting, want_diarize: bool) -> bool {
    let has_room = resolved
        .inputs
        .iter()
        .any(|i| i.role == InputRole::Room);
    match resolved.meeting.alignment.reference {
        AlignmentReference::None | AlignmentReference::Timeline => false,
        AlignmentReference::Mix => has_room,
        AlignmentReference::Auto => has_room && !want_diarize,
    }
}

/// Ensure room media is available as a merge input (path or `{branch}.prepared`).
fn append_mix_ref(
    steps: &mut Vec<Step>,
    resolved: &ResolvedMeeting,
    options: &BuildOptions,
) -> Result<String, PlanError> {
    let src = resolved
        .inputs
        .iter()
        .find(|i| i.role == InputRole::Room)
        .or_else(|| {
            resolved
                .timeline_sources
                .iter()
                .map(|&i| &resolved.inputs[i])
                .find(|i| i.purposes.contains(&InputPurpose::Timeline))
        })
        .ok_or_else(|| {
            PlanError::Usage(
                "alignment reference mix requires a room input".into(),
            )
        })?;

    // Reuse prepared artifact when room was already preprocessed for transcript.
    let already_prepared = vd_pipeline::is_video_path(&src.path)
        && resolved
            .text_sources
            .iter()
            .any(|&i| resolved.inputs[i].branch_id == src.branch_id);
    if already_prepared {
        return Ok(format!("{}.prepared", src.branch_id));
    }

    // Prefer stable id `room.mix` when media_input_ref returns a path (audio file).
    let media = preprocess::media_input_ref(steps, src, options)?;
    if media.contains('/') || media.ends_with(".wav") || media.ends_with(".mp3")
        || media.ends_with(".m4a") || media.ends_with(".ogg") || media.ends_with(".flac")
    {
        // Filesystem path — register under a stable step id so merge can depend on it
        // and options.mix is an artifact id, not a raw path (paths still allowed).
        Ok(media)
    } else {
        Ok(media)
    }
}

fn transcribe_options(options: &BuildOptions) -> std::collections::BTreeMap<String, ArgValue> {
    let mut o = std::collections::BTreeMap::new();
    let engine = options
        .transcribe
        .engine
        .clone()
        .unwrap_or_else(|| "gigaam".into());
    o.insert("engine".into(), ArgValue::String(engine));
    if let Some(m) = &options.transcribe.model {
        o.insert("model".into(), ArgValue::String(m.clone()));
    }
    if let Some(d) = &options.transcribe.device {
        o.insert("device".into(), ArgValue::String(d.clone()));
    }
    if options.transcribe.overwrite {
        o.insert("overwrite".into(), ArgValue::Bool(true));
    }
    o
}

fn overwrite_opt(options: &BuildOptions) -> std::collections::BTreeMap<String, ArgValue> {
    let mut o = std::collections::BTreeMap::new();
    if options.transcribe.overwrite {
        o.insert("overwrite".into(), ArgValue::Bool(true));
    }
    o
}
