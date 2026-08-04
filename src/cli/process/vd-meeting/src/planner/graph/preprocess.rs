//! Optional media preprocess (video → extract-audio; speed; alignment pad).
//!
//! URL sources are materialized by `vd-input` before planning (ADR 0008).

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use vd_pipeline::{default_preprocess_filters, is_video_path, ArgValue, Capability, Step};

use crate::model::{AlignmentMode, BuildOptions, InputRole};
use crate::planner::normalize::{ResolvedInput, ResolvedMeeting};
use crate::planner::PlanError;

const PAD_EPSILON_SEC: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PadSide {
    /// Leading silence (`pad-start`) — `longest` / `start`.
    #[default]
    Start,
    /// Trailing silence (`pad-end`) — `end`.
    End,
}

impl PadSide {
    fn filter_op(self) -> &'static str {
        match self {
            Self::Start => "pad-start",
            Self::End => "pad-end",
        }
    }
}

/// Per-branch silence pad to match the longest media source.
#[derive(Debug, Clone, Default)]
pub struct AlignPads {
    pub side: PadSide,
    /// `branch_id` → seconds of silence to insert.
    pub by_branch: BTreeMap<String, f64>,
}

impl AlignPads {
    pub fn pad_for(&self, branch_id: &str) -> Option<f64> {
        self.by_branch
            .get(branch_id)
            .copied()
            .filter(|&s| s > PAD_EPSILON_SEC)
    }
}

fn pad_side_for_mode(mode: AlignmentMode) -> PadSide {
    match mode {
        AlignmentMode::Longest | AlignmentMode::Start => PadSide::Start,
        AlignmentMode::End => PadSide::End,
    }
}

/// Probe participant + room media; pad shorter tracks to the longest duration.
pub fn compute_align_pads(resolved: &ResolvedMeeting) -> AlignPads {
    let side = pad_side_for_mode(resolved.meeting.alignment.mode);
    let mut durs: Vec<(String, f64)> = Vec::new();

    let mut seen = std::collections::HashSet::new();
    for &idx in resolved
        .text_sources
        .iter()
        .chain(resolved.timeline_sources.iter())
    {
        let src = &resolved.inputs[idx];
        if src.role == InputRole::Context {
            continue;
        }
        if src.path.as_os_str().is_empty() || !src.path.exists() {
            continue;
        }
        if !seen.insert(src.branch_id.clone()) {
            continue;
        }
        if let Some(d) = probe_duration_sec(&src.path) {
            if d.is_finite() && d > 0.0 {
                durs.push((src.branch_id.clone(), d));
            }
        }
    }

    // Room may not be in text/timeline lists when reference=mix only — still include.
    if let Some(room) = resolved.inputs.iter().find(|i| i.role == InputRole::Room) {
        if seen.insert(room.branch_id.clone())
            && !room.path.as_os_str().is_empty()
            && room.path.exists()
        {
            if let Some(d) = probe_duration_sec(&room.path) {
                if d.is_finite() && d > 0.0 {
                    durs.push((room.branch_id.clone(), d));
                }
            }
        }
    }

    let mut pads = AlignPads {
        side,
        by_branch: BTreeMap::new(),
    };
    if durs.len() < 2 {
        return pads;
    }
    let max_dur = durs.iter().map(|(_, d)| *d).fold(0.0_f64, f64::max);
    for (bid, d) in durs {
        let pad = max_dur - d;
        if pad > PAD_EPSILON_SEC {
            pads.by_branch.insert(bid, pad);
        }
    }
    pads
}

pub fn will_preprocess(src: &ResolvedInput, options: &BuildOptions, pads: &AlignPads) -> bool {
    is_video_path(&src.path)
        || options.transcribe.speed.is_some()
        || pads.pad_for(&src.branch_id).is_some()
}

/// Append preprocess when video, speed, and/or alignment pad require it.
/// Otherwise return the filesystem path for direct ASR / diarize input.
pub fn media_input_ref(
    steps: &mut Vec<Step>,
    src: &ResolvedInput,
    options: &BuildOptions,
    pads: &AlignPads,
) -> Result<String, PlanError> {
    let path = src.path.display().to_string();
    let speed = options.transcribe.speed;
    if let Some(factor) = speed {
        if !(0.25..=4.0).contains(&factor) {
            return Err(PlanError::Usage(format!(
                "speed must be between 0.25 and 4.0 (got {factor})"
            )));
        }
    }
    let pad_sec = pads.pad_for(&src.branch_id);
    if !will_preprocess(src, options, pads) {
        return Ok(path);
    }

    let bid = &src.branch_id;
    let prep_id = format!("{bid}.prepared");
    let (mut provider, mut filters) = default_preprocess_filters(&src.path, speed);
    // Meeting clocks must stay piecewise-correct. Default ASR chain includes
    // `trim-silence`, but preprocess only emits a *uniform* TimeMap today —
    // silenceremove then linearly stretches compacted speech across the full
    // original duration and wrecks participant timestamps / mix subtract
    // (ADR 0016). Drop it until piecewise silence maps exist.
    strip_trim_silence(&mut filters);

    // Alignment pad and any non-stub filter need a real ffmpeg backend.
    if pad_sec.is_some() || is_video_path(&src.path) || speed.is_some() {
        provider = "ffmpeg".into();
    }
    if let Some(sec) = pad_sec {
        insert_pad_filter(&mut filters, pads.side, sec);
    }

    let mut preprocess_opts = BTreeMap::new();
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

/// Insert `pad-start` / `pad-end` into the filter chain (prefer after normalize
/// inputs that rewrite duration, before normalize when possible).
fn insert_pad_filter(filters: &mut Vec<ArgValue>, side: PadSide, sec: f64) {
    let pad = {
        let mut m = BTreeMap::new();
        m.insert("type".into(), ArgValue::String(side.filter_op().into()));
        m.insert("duration_sec".into(), ArgValue::Number(sec));
        ArgValue::Map(m)
    };
    let before_norm = filters.iter().position(|f| {
        f.as_map()
            .and_then(|m| m.get("type").or_else(|| m.get("operation")))
            .and_then(ArgValue::as_string)
            .as_deref()
            == Some("normalize")
    });
    if let Some(i) = before_norm {
        filters.insert(i, pad);
    } else {
        filters.push(pad);
    }
}

fn strip_trim_silence(filters: &mut Vec<ArgValue>) {
    filters.retain(|f| {
        f.as_map()
            .and_then(|m| m.get("type").or_else(|| m.get("operation")))
            .and_then(ArgValue::as_string)
            .as_deref()
            != Some("trim-silence")
    });
}

fn probe_duration_sec(path: &Path) -> Option<f64> {
    let bin = std::env::var_os("VD_FFPROBE")
        .map(std::path::PathBuf::from)
        .or_else(|| which_bin("ffprobe"))
        .unwrap_or_else(|| std::path::PathBuf::from("ffprobe"));
    let out = Command::new(&bin)
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
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse().ok()
}

fn which_bin(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
