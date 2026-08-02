//! Materialize InputSource.url via vd-input before DAG build (ADR 0008).

use std::path::Path;

use vd_input::{resolve, InputSource as WireSource, ResolveContext, SubtitlePolicy};

use crate::model::{InputRole, MeetingRequest};
use crate::planner::PlanError;

/// Resolve any `url` inputs into local audio paths. Path-only inputs pass through.
pub fn materialize_request(
    request: &MeetingRequest,
    data_dir: &Path,
) -> Result<MeetingRequest, PlanError> {
    let mut out = request.clone();
    for input in &mut out.inputs {
        if input.role == InputRole::Context {
            continue;
        }
        let Some(url) = input
            .url
            .as_deref()
            .map(str::trim)
            .filter(|u| !u.is_empty())
        else {
            continue;
        };
        let wire = WireSource {
            url: Some(url.to_string()),
            ..Default::default()
        };
        let mut ctx = ResolveContext::new(data_dir);
        ctx.overwrite = true;
        ctx.provider_hint = input.provider.as_deref();
        if let Some(s) = &input.subtitles {
            ctx.subtitles = SubtitlePolicy::parse(s).map_err(PlanError::Usage)?;
        }
        let resolved = resolve(&wire, &ctx, None).map_err(|e| PlanError::Usage(e.to_string()))?;
        let audio = resolved
            .require_audio()
            .map_err(|e| PlanError::Usage(e.to_string()))?
            .clone();
        input.path = audio;
        input.url = None;
        input.subtitles = None;
        input.provider = None;
    }
    Ok(out)
}
