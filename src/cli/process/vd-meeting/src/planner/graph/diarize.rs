//! Diarize branch when policy + a timeline-purpose source allow it.

use vd_pipeline::{Capability, Step};

use super::overwrite_opt;
use crate::model::{BuildOptions, InputPurpose};
use crate::planner::normalize::ResolvedMeeting;
use crate::planner::PlanError;

pub fn append_diarize(
    steps: &mut Vec<Step>,
    resolved: &ResolvedMeeting,
    options: &BuildOptions,
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

    let id = "timeline".to_string();
    let mut step = Step::new(Capability::Diarize);
    step.id = Some(id.clone());
    step.input = Some(src.path.display().to_string());
    step.options = overwrite_opt(options);
    steps.push(step);
    Ok(id)
}
