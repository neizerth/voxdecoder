//! Build Job DAG from ResolvedMeeting.

mod diarize;
mod merge;
mod transcript;

use vd_pipeline::{
    ArgValue, Capability, Job, JobContext, JobInput, JobOutput, Step,
};

use super::normalize::ResolvedMeeting;
use super::PlanError;
use crate::model::{BuildOptions, DiarizationEnabled, InputRole};

pub fn build_job(
    resolved: &ResolvedMeeting,
    options: &BuildOptions,
) -> Result<Job, PlanError> {
    let mut steps: Vec<Step> = Vec::new();

    if resolved.has_context {
        if let Some(ctx) = resolved.inputs.iter().find(|i| i.role == InputRole::Context) {
            let mut step = Step::new(Capability::PrepareContext);
            step.id = Some("assets".into());
            step.input = Some(ctx.path.display().to_string());
            steps.push(step);
        }
    }

    let mut text_ids: Vec<String> = Vec::new();
    for &idx in &resolved.text_sources {
        let src = &resolved.inputs[idx];
        let final_id = transcript::append_branch(&mut steps, src, options)?;
        text_ids.push(final_id);
    }

    let want_diarize = should_diarize(resolved);
    let timeline_id = if want_diarize {
        Some(diarize::append_diarize(&mut steps, resolved, options)?)
    } else {
        None
    };

    if text_ids.is_empty() && timeline_id.is_none() {
        return Err(PlanError::Usage(
            "planner produced no mergeable artifacts".into(),
        ));
    }

    merge::append_merge(&mut steps, resolved, &text_ids, timeline_id.as_deref(), options)?;

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

    // Prefer merged audio as Job.input.audio when present (diarize default).
    let audio = resolved
        .inputs
        .iter()
        .find(|i| i.role == InputRole::Merged)
        .or_else(|| {
            resolved
                .inputs
                .iter()
                .find(|i| i.role == InputRole::Participant)
        })
        .map(|i| i.path.clone());

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
        max_parallel: options.executor.max_parallel,
        resources: options.executor.resources.clone(),
        steps,
    })
}

fn should_diarize(resolved: &ResolvedMeeting) -> bool {
    match resolved.meeting.diarization.enabled {
        DiarizationEnabled::False => false,
        DiarizationEnabled::True | DiarizationEnabled::Auto => resolved.has_merged,
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
