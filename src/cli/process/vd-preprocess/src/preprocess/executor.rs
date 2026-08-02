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

    let mut extras = Vec::new();
    let mut timemap = None;
    if filters_change_time(&req.filters) {
        let in_dur = probe_duration(&req.input)?;
        let out_dur = probe_duration(&planned.output)?;
        if in_dur > 0.0 && out_dur > 0.0 {
            let map = vd_artifact::TimeMap::uniform(out_dur, in_dur);
            let path = timemap_sidecar_path(&planned.output);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| PreprocessError::Other(e.to_string()))?;
            }
            let body = serde_json::to_string_pretty(&map)
                .map_err(|e| PreprocessError::Other(e.to_string()))?;
            std::fs::write(&path, body).map_err(|e| PreprocessError::Other(e.to_string()))?;
            extras.push(PreparedMedia {
                id: Some("timemap".into()),
                path: path.clone(),
            });
            timemap = Some(path);
        }
    }

    Ok(PreprocessResult {
        output: PreparedMedia {
            id: None,
            path: planned.output,
        },
        extras,
        timemap,
    })
}

fn filters_change_time(filters: &[FilterSpec]) -> bool {
    filters.iter().any(|f| {
        matches!(
            f.operation.as_str(),
            "speed" | "trim-silence" | "trim" | "chunk"
        )
    })
}

fn timemap_sidecar_path(media: &Path) -> PathBuf {
    let stem = media
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prepared");
    let parent = media.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}.timemap.json"))
}

/// Best-effort media duration in seconds (ffprobe, else ffmpeg stderr).
fn probe_duration(path: &Path) -> Result<f64, PreprocessError> {
    if let Some(ffprobe) = find_ffprobe() {
        if let Ok(out) = std::process::Command::new(&ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path)
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(v) = s.parse::<f64>() {
                    if v.is_finite() && v > 0.0 {
                        return Ok(v);
                    }
                }
            }
        }
    }

    // Fallback: `ffmpeg -i` prints Duration on stderr.
    let ffmpeg = provider::find_ffmpeg_for_probe().unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let out = std::process::Command::new(&ffmpeg)
        .arg("-i")
        .arg(path)
        .output()
        .map_err(|e| PreprocessError::Other(format!("ffmpeg probe: {e}")))?;
    let err = String::from_utf8_lossy(&out.stderr);
    parse_ffmpeg_duration(&err).ok_or_else(|| {
        PreprocessError::Other(format!(
            "could not probe duration for {}",
            path.display()
        ))
    })
}

fn parse_ffmpeg_duration(stderr: &str) -> Option<f64> {
    // Duration: 00:01:46.16
    let idx = stderr.find("Duration: ")?;
    let rest = &stderr[idx + "Duration: ".len()..];
    let token = rest.split(',').next()?.trim();
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

fn find_ffprobe() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VD_FFPROBE") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(ffmpeg) = std::env::var("VD_FFMPEG") {
        let ffmpeg = PathBuf::from(ffmpeg);
        if let Some(parent) = ffmpeg.parent() {
            let sibling = parent.join("ffprobe");
            if sibling.is_file() {
                return Some(sibling);
            }
        }
    }
    which_bin("ffprobe")
}

fn which_bin(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(bin);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
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
