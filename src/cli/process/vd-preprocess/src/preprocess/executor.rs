//! Plan and execute preprocess requests.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::chain::expand_and_validate;
use super::filter::{FilterSpec, RawFilter};
use super::provider::{self, describe_step};
use super::result::{PreparedMedia, PreprocessResult};
use super::PreprocessError;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreprocessRequest {
    pub input: PathBuf,
    pub filters: Vec<FilterSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub overwrite: bool,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedFilter {
    pub index: usize,
    pub provider: String,
    pub operation: String,
    pub input: PathBuf,
    pub output: PathBuf,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub detail: serde_json::Map<String, serde_json::Value>,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub default_provider: String,
    pub input: PathBuf,
    pub output: PathBuf,
    pub steps: Vec<PlannedFilter>,
}

/// Expand raw filters (from chain file merge) into a request.
pub fn request_from_raw(
    input: PathBuf,
    raw: Vec<RawFilter>,
    default_provider: &str,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    overwrite: bool,
) -> Result<PreprocessRequest, PreprocessError> {
    let filters = expand_and_validate(raw, default_provider)?;
    Ok(PreprocessRequest {
        input,
        filters,
        provider: Some(default_provider.to_string()),
        output,
        output_dir,
        overwrite,
    })
}

pub fn plan(req: &PreprocessRequest) -> Result<ExecutionPlan, PreprocessError> {
    if req.filters.is_empty() {
        return Err(PreprocessError::Usage("no filters specified".into()));
    }
    if !req.input.exists() {
        return Err(PreprocessError::NotFound(format!(
            "input missing: {}",
            req.input.display()
        )));
    }

    let default_provider = req
        .provider
        .clone()
        .unwrap_or_else(|| "stub".into());
    let final_out = resolve_output_path(req)?;
    let work_dir = final_out
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut steps = Vec::with_capacity(req.filters.len());
    let mut current = req.input.clone();
    for (i, filter) in req.filters.iter().enumerate() {
        // Plan-time: allow ffmpeg even if binary missing (argv still described).
        if filter.provider != "ffmpeg" && filter.provider != "stub" {
            provider::provider_available_for_plan(&filter.provider)?;
        }
        let out = if i + 1 == req.filters.len() {
            final_out.clone()
        } else {
            work_dir.join(format!(
                ".vd-preprocess-{}-{}.tmp{}",
                i,
                filter.operation,
                extension_hint(&final_out)
            ))
        };
        let detail_map = describe_step(filter, &current, &out);
        let detail: serde_json::Map<String, serde_json::Value> =
            detail_map.into_iter().collect();
        steps.push(PlannedFilter {
            index: i,
            provider: filter.provider.clone(),
            operation: filter.operation.clone(),
            input: current.clone(),
            output: out.clone(),
            detail,
        });
        current = out;
    }

    Ok(ExecutionPlan {
        default_provider,
        input: req.input.clone(),
        output: final_out,
        steps,
    })
}

pub fn execute(req: &PreprocessRequest) -> Result<PreprocessResult, PreprocessError> {
    let planned = plan(req)?;
    if planned.output.exists() && !req.overwrite {
        return Err(PreprocessError::Usage(format!(
            "output exists (pass --overwrite): {}",
            planned.output.display()
        )));
    }

    let mut temps = Vec::new();
    for (i, step) in planned.steps.iter().enumerate() {
        let filter = &req.filters[step.index];
        let backend = provider::resolve_provider(&filter.provider)?;
        backend.apply(filter, &step.input, &step.output)?;
        if i + 1 < planned.steps.len() {
            temps.push(step.output.clone());
        }
    }

    for t in temps {
        let _ = std::fs::remove_file(t);
    }

    Ok(PreprocessResult {
        output: PreparedMedia {
            id: None,
            path: planned.output,
        },
        extras: Vec::new(),
    })
}

fn resolve_output_path(req: &PreprocessRequest) -> Result<PathBuf, PreprocessError> {
    if let Some(o) = &req.output {
        return Ok(o.clone());
    }
    let stem = req
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prepared");
    let ext = req
        .input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    let name = format!("{stem}.prepared.{ext}");
    if let Some(dir) = &req.output_dir {
        Ok(dir.join(name))
    } else if let Some(parent) = req.input.parent() {
        Ok(parent.join(name))
    } else {
        Ok(PathBuf::from(name))
    }
}

fn extension_hint(final_out: &Path) -> String {
    final_out
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default()
}
