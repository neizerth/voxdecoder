//! One transcript branch: (preprocess?) → transcribe → fix-casing → fix-asr → fix-terms.

use vd_pipeline::{Capability, Step};

use super::preprocess::media_input_ref;
use super::{overwrite_opt, transcribe_options};
use crate::model::BuildOptions;
use crate::planner::normalize::ResolvedInput;
use crate::planner::PlanError;

/// Append branch steps; returns final text artifact id (`{branch}.text`).
pub fn append_branch(
    steps: &mut Vec<Step>,
    src: &ResolvedInput,
    options: &BuildOptions,
) -> Result<String, PlanError> {
    let bid = &src.branch_id;
    let media = media_input_ref(steps, src, options)?;

    let tid = format!("{bid}.transcript");
    let mut t = Step::new(Capability::Transcribe);
    t.id = Some(tid.clone());
    t.input = Some(media);
    t.options = transcribe_options(options);
    steps.push(t);

    let cased = format!("{bid}.cased");
    let mut c = Step::new(Capability::FixCasing);
    c.id = Some(cased.clone());
    c.inputs = vec![tid];
    c.options = overwrite_opt(options);
    steps.push(c);

    let asr = format!("{bid}.asr");
    let mut a = Step::new(Capability::FixAsr);
    a.id = Some(asr.clone());
    a.inputs = vec![cased];
    a.options = overwrite_opt(options);
    steps.push(a);

    let text = format!("{bid}.text");
    let mut f = Step::new(Capability::FixTerms);
    f.id = Some(text.clone());
    f.inputs = vec![asr];
    f.options = overwrite_opt(options);
    steps.push(f);

    Ok(text)
}
