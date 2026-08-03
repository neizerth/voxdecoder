//! One transcript branch: (preprocess?) → transcribe → fix-casing → fix-asr →
//! fix-disfluency → fix-terms → fix-layout (same order as `vd-pipeline` default).

use vd_pipeline::{Capability, Step};

use super::preprocess::{media_input_ref, AlignPads};
use super::{overwrite_opt, transcribe_options};
use crate::model::BuildOptions;
use crate::planner::normalize::ResolvedInput;
use crate::planner::PlanError;

/// Append branch steps; returns final text artifact id (`{branch}.text`).
pub fn append_branch(
    steps: &mut Vec<Step>,
    src: &ResolvedInput,
    options: &BuildOptions,
    pads: &AlignPads,
) -> Result<String, PlanError> {
    let bid = &src.branch_id;
    let media = media_input_ref(steps, src, options, pads)?;

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

    let disfluency = format!("{bid}.disfluency");
    let mut d = Step::new(Capability::FixDisfluency);
    d.id = Some(disfluency.clone());
    d.inputs = vec![asr];
    d.options = overwrite_opt(options);
    steps.push(d);

    let terms = format!("{bid}.terms");
    let mut f = Step::new(Capability::FixTerms);
    f.id = Some(terms.clone());
    f.inputs = vec![disfluency];
    f.options = overwrite_opt(options);
    steps.push(f);

    let text = format!("{bid}.text");
    let mut layout = Step::new(Capability::FixLayout);
    layout.id = Some(text.clone());
    layout.inputs = vec![terms];
    layout.options = overwrite_opt(options);
    steps.push(layout);

    Ok(text)
}
