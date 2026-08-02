//! Apply TimeMap to timeline-bearing step outputs (segments / SRT).

use std::fs;
use std::path::{Path, PathBuf};

use vd_artifact::{remap_segments_json, remap_srt_file, TimeMap};

use super::ExecError;

pub fn load_timemap(path: &Path) -> Result<TimeMap, ExecError> {
    let raw = fs::read_to_string(path).map_err(|e| ExecError::Step(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| ExecError::Step(format!("invalid TimeMap: {e}")))
}

/// Remap known timeline sidecars next to / listed in step outputs.
pub fn remap_timeline_outputs(
    primary: &Path,
    named: &std::collections::BTreeMap<String, PathBuf>,
    map: &TimeMap,
) -> Result<(), ExecError> {
    let mut targets: Vec<PathBuf> = named
        .values()
        .filter(|p| is_timeline_path(p))
        .cloned()
        .collect();

    // Convention sidecars next to primary transcript.
    let stem = primary
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let parent = primary.parent().unwrap_or_else(|| Path::new("."));
    for name in [
        format!("{stem}.segments.json"),
        format!("{stem}.srt"),
        format!("{stem}.vtt"),
    ] {
        let p = parent.join(name);
        if p.is_file() && !targets.iter().any(|t| t == &p) {
            targets.push(p);
        }
    }

    if primary
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("srt"))
        && !targets.iter().any(|t| t == primary)
    {
        targets.push(primary.to_path_buf());
    }

    for path in targets {
        remap_one(&path, map)?;
    }
    Ok(())
}

fn is_timeline_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.ends_with(".segments.json")
        || name.ends_with(".srt")
        || name.ends_with(".vtt")
        || name.contains("timemap")
}

fn remap_one(path: &Path, map: &TimeMap) -> Result<(), ExecError> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.contains("timemap") {
        return Ok(());
    }
    if name.ends_with(".segments.json") || (name.ends_with(".json") && name.contains("segment")) {
        remap_segments_json(path, map).map_err(|e| ExecError::Step(e.to_string()))?;
        return Ok(());
    }
    if name.ends_with(".srt") {
        remap_srt_file(path, map).map_err(|e| ExecError::Step(e.to_string()))?;
    }
    // VTT: reuse SRT-ish arrow lines when present; skip otherwise for now.
    Ok(())
}
