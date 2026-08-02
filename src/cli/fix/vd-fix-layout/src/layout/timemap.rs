//! Optional TimeMap binding (abstract source, not a path promise).

use std::fs;
use std::path::Path;

use crate::types::{TimeMap, TimeMapSource};

#[derive(Debug, Clone)]
pub struct BoundTimeMap {
    pub source: TimeMapSource,
    pub map: Option<TimeMap>,
}

/// Bind TimeMap from explicit CLI path, else discover a sibling sidecar.
pub fn bind_timemap(input: &Path, cli_path: Option<&Path>) -> BoundTimeMap {
    if let Some(p) = cli_path {
        return load_map(p).map_or(
            BoundTimeMap {
                source: TimeMapSource::None,
                map: None,
            },
            |map| BoundTimeMap {
                source: TimeMapSource::Cli,
                map: Some(map),
            },
        );
    }

    if let Some(p) = discover_sidecar(input) {
        if let Some(map) = load_map(&p) {
            return BoundTimeMap {
                source: TimeMapSource::Artifact,
                map: Some(map),
            };
        }
    }

    BoundTimeMap {
        source: TimeMapSource::None,
        map: None,
    }
}

fn discover_sidecar(input: &Path) -> Option<std::path::PathBuf> {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem()?.to_str()?;
    let stem = stem.strip_suffix(".fixed").unwrap_or(stem);
    let candidates = [
        parent.join(format!("{stem}.timemap.json")),
        parent.join(format!("{stem}.prepared.timemap.json")),
        parent.join("prepared.timemap.json"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn load_map(path: &Path) -> Option<TimeMap> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Gaps (seconds) between consecutive original TimeMap segments — pause hints.
pub fn pause_gaps(map: &TimeMap) -> Vec<f64> {
    let mut gaps = Vec::new();
    for w in map.segments.windows(2) {
        let gap = w[1].original.start - w[0].original.end;
        if gap > 0.0 {
            gaps.push(gap);
        }
    }
    gaps
}

/// Whether pause evidence suggests preferring more breaks (long trimmed silences).
pub fn prefers_relaxed_breaks(map: &TimeMap) -> bool {
    let gaps = pause_gaps(map);
    if gaps.is_empty() {
        return false;
    }
    let long = gaps.iter().filter(|&&g| g >= 1.5).count();
    long >= 2 || gaps.iter().any(|&g| g >= 3.0)
}
