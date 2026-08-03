//! Durable execution report (profiling / audit) — separate from `vd-progress` UI events.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::job::{ArgValue, ResolvedStep};

pub const REPORT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobReportStatus {
    Ok,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepReportStatus {
    Ok,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactStat {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

impl ArtifactStat {
    pub fn from_path(path: &Path) -> Self {
        let bytes = fs::metadata(path).ok().map(|m| m.len());
        Self {
            path: path.display().to_string(),
            bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseReport {
    pub name: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepReport {
    pub id: String,
    pub capability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub status: StepReportStatus,
    pub queued_at: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<PhaseReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<ArtifactStat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<ArtifactStat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    pub status: JobReportStatus,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    /// Longest leaf duration (proxy for critical path until full DAG metrics).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_path_ms: Option<u64>,
    /// `work_sum / wall_duration`, capped at 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_efficiency: Option<f64>,
    pub steps: Vec<StepReport>,
}

impl ExecutionReport {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn write_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        let body = self.to_json_pretty().map_err(|e| e.to_string())?;
        fs::write(path, body).map_err(|e| e.to_string())
    }
}

pub fn duration_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

pub fn format_rfc3339(t: SystemTime) -> String {
    let Ok(dur) = t.duration_since(UNIX_EPOCH) else {
        return "1970-01-01T00:00:00.000Z".into();
    };
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let (y, mo, d, h, mi, s) = civil_from_unix(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{millis:03}Z")
}

/// Howard Hinnant civil_from_days + clock fields.
fn civil_from_unix(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = (rem / 3600) as u32;
    let mi = ((rem % 3600) / 60) as u32;
    let s = (rem % 60) as u32;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64 + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, h, mi, s)
}

pub fn step_id(step: &ResolvedStep) -> String {
    step.id
        .clone()
        .unwrap_or_else(|| format!("{}-{}", step.capability.as_str(), step.index))
}

pub fn backend_from_options(
    options: &std::collections::BTreeMap<String, ArgValue>,
) -> Option<String> {
    if let Some(v) = options.get("engine").and_then(ArgValue::as_string) {
        return Some(v);
    }
    if let Some(v) = options.get("provider").and_then(ArgValue::as_string) {
        return Some(v);
    }
    match options.get("backend") {
        Some(ArgValue::String(s)) => Some(s.clone()),
        Some(ArgValue::Map(m)) => m.get("provider").and_then(ArgValue::as_string),
        _ => None,
    }
}

pub fn model_from_options(
    options: &std::collections::BTreeMap<String, ArgValue>,
) -> Option<String> {
    if let Some(v) = options.get("model").and_then(ArgValue::as_string) {
        return Some(v);
    }
    if let Some(ArgValue::Map(m)) = options.get("backend") {
        return m.get("model").and_then(ArgValue::as_string);
    }
    None
}

pub fn make_step_report(
    step: &ResolvedStep,
    status: StepReportStatus,
    started: SystemTime,
    finished: SystemTime,
    duration: Duration,
    input: Option<&Path>,
    outputs: &[PathBuf],
) -> StepReport {
    let inputs = input.map(ArtifactStat::from_path).into_iter().collect();
    let outputs = outputs.iter().map(|p| ArtifactStat::from_path(p)).collect();
    let ts = format_rfc3339(started);
    StepReport {
        id: step_id(step),
        capability: step.capability.as_str().to_string(),
        name: step.name.clone(),
        status,
        queued_at: ts.clone(),
        started_at: ts,
        finished_at: format_rfc3339(finished),
        duration_ms: duration_ms(duration),
        backend: backend_from_options(&step.options),
        model: model_from_options(&step.options),
        phases: Vec::new(),
        inputs,
        outputs,
    }
}
