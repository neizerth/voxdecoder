//! Diarize branch when policy + a timeline-purpose source allow it.

use vd_pipeline::{Capability, Step};

use super::overwrite_opt;
use super::preprocess::{media_input_ref, will_preprocess, AlignPads};
use crate::model::{BuildOptions, InputPurpose};
use crate::planner::normalize::ResolvedMeeting;
use crate::planner::PlanError;

pub fn append_diarize(
    steps: &mut Vec<Step>,
    resolved: &ResolvedMeeting,
    options: &BuildOptions,
    pads: &AlignPads,
) -> Result<String, PlanError> {
    let src = resolved
        .timeline_sources
        .iter()
        .map(|&i| &resolved.inputs[i])
        .find(|i| i.purposes.contains(&InputPurpose::Timeline))
        .ok_or_else(|| {
            PlanError::Usage(
                "diarize requires an audio input with purpose timeline (usually role: room)".into(),
            )
        })?;

    // Same branch may already have `{bid}.prepared` from the transcript path.
    let already_prepared = will_preprocess(src, options, pads)
        && resolved
            .text_sources
            .iter()
            .any(|&i| resolved.inputs[i].branch_id == src.branch_id);
    let media = if already_prepared {
        format!("{}.prepared", src.branch_id)
    } else {
        media_input_ref(steps, src, options, pads)?
    };

    let id = "timeline".to_string();
    let mut step = Step::new(Capability::Diarize);
    step.id = Some(id.clone());
    step.input = Some(media);
    step.options = overwrite_opt(options);
    steps.push(step);
    Ok(id)
}
