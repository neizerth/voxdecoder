//! Optional media preprocess (video → extract-audio via ffmpeg).

use vd_pipeline::{
    default_preprocess_filters, is_video_path, ArgValue, Capability, Step,
};

use crate::model::BuildOptions;
use crate::planner::normalize::ResolvedInput;
use crate::planner::PlanError;

/// If `src` is video, append preprocess and return the prepared artifact id.
/// Otherwise return the filesystem path for direct ASR / diarize input.
pub fn media_input_ref(
    steps: &mut Vec<Step>,
    src: &ResolvedInput,
    options: &BuildOptions,
) -> Result<String, PlanError> {
    let path = src.path.display().to_string();
    if !is_video_path(&src.path) {
        return Ok(path);
    }

    let bid = &src.branch_id;
    let prep_id = format!("{bid}.prepared");
    let (provider, filters) = default_preprocess_filters(&src.path, None);

    let mut preprocess_opts = std::collections::BTreeMap::new();
    preprocess_opts.insert("provider".into(), ArgValue::String(provider));
    preprocess_opts.insert("filters".into(), ArgValue::List(filters));
    if options.transcribe.overwrite {
        preprocess_opts.insert("overwrite".into(), ArgValue::Bool(true));
    }

    let mut p = Step::new(Capability::Preprocess);
    p.id = Some(prep_id.clone());
    p.input = Some(path);
    p.options = preprocess_opts;
    steps.push(p);
    Ok(prep_id)
}
