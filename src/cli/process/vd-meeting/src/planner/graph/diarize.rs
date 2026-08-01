//! Diarize branch when policy + merged source allow it.

use vd_pipeline::{Capability, Step};

use super::overwrite_opt;
use crate::model::{BuildOptions, InputRole};
use crate::planner::normalize::ResolvedMeeting;
use crate::planner::PlanError;

pub fn append_diarize(
    steps: &mut Vec<Step>,
    resolved: &ResolvedMeeting,
    options: &BuildOptions,
) -> Result<String, PlanError> {
    let merged = resolved
        .inputs
        .iter()
        .find(|i| i.role == InputRole::Merged)
        .ok_or_else(|| PlanError::Usage("diarize requires a merged input".into()))?;

    let id = "timeline".to_string();
    let mut step = Step::new(Capability::Diarize);
    step.id = Some(id.clone());
    step.input = Some(merged.path.display().to_string());
    step.options = overwrite_opt(options);
    steps.push(step);
    Ok(id)
}
