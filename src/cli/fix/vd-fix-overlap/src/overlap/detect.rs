//! Duplicate-speech detection algorithm (ADR 0012 §2 "Detection signals").
//!
//! Deliberately takes its own minimal input struct (`Utterance`) instead of
//! a `vd-artifact` type: no shared crate exposes speaker + timestamp + text
//! together today (see `STRUCTURE.md`). This module has no I/O and no
//! knowledge of any on-disk artifact shape — callers are responsible for
//! turning a real transcript into `Utterance`s.

/// One diarized utterance: speaker + time range + text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utterance {
    pub speaker: String,
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

/// Detection thresholds (ADR 0012 §2: "Corrections require high
/// confidence. If uncertain: preserve both copies." — both knobs default
/// conservative).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectOptions {
    /// Normalized text similarity in `[0.0, 1.0]` at/above which two spans
    /// from different speakers count as "near-identical" / "high lexical
    /// similarity". Exact matches (after normalization) are always flagged
    /// regardless of this threshold.
    pub similarity_threshold: f64,
    /// Max gap in milliseconds between the closer edges of two time ranges
    /// that still counts as "short temporal distance" when the ranges do
    /// not overlap outright.
    pub max_gap_ms: u64,
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
            max_gap_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateKind {
    /// Identical after normalization (case, whitespace, punctuation).
    Exact,
    /// Below-identity but at/above `similarity_threshold`.
    Near,
}

/// What to do about `drop`'s turn (ADR 0012 §2 "partial duplicates": trim
/// the duplicated fragment when deterministic, don't destroy unique
/// content).
///
/// Only ever `TrimTo` when `drop`'s text case-insensitively *contains*
/// `keep`'s text as a clean prefix or suffix — anything less certain (a
/// fuzzy edit-distance match with no clean boundary) falls back to
/// `RemoveWhole`, matching this module's original, already-tested behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrimAction {
    /// Delete the whole `drop` turn — it has no content beyond what `keep`
    /// already has.
    RemoveWhole,
    /// `drop` fully contains `keep`'s text as a prefix/suffix plus a real
    /// unique remainder. Deleting the whole turn would lose that
    /// remainder, so keep the turn and rewrite its text to just this.
    TrimTo(String),
}

/// One detected duplicate pair.
///
/// `keep` / `drop` are input indices, ordered by start time (earlier
/// survives) with a stable index tie-break — this module only
/// *recommends*; it never mutates anything (ADR 0012 §2 "Never delete
/// unique speech").
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicatePair {
    pub keep: usize,
    pub drop: usize,
    pub kind: DuplicateKind,
    pub similarity: f64,
    pub trim: TrimAction,
}

/// Find duplicated speech across *different* speakers. Same-speaker repeats
/// are out of scope — that is not diarization overlap (ADR 0012 §2
/// motivation: "the same utterance to multiple speakers").
pub fn detect_duplicates(utterances: &[Utterance], opts: &DetectOptions) -> Vec<DuplicatePair> {
    let normalized: Vec<String> = utterances.iter().map(|u| normalize(&u.text)).collect();
    let mut out = Vec::new();
    for i in 0..utterances.len() {
        for j in (i + 1)..utterances.len() {
            let a = &utterances[i];
            let b = &utterances[j];
            if a.speaker == b.speaker {
                continue;
            }
            if !temporally_close(a, b, opts.max_gap_ms) {
                continue;
            }
            let na = &normalized[i];
            let nb = &normalized[j];
            if na.is_empty() || nb.is_empty() {
                continue;
            }
            let (kind, similarity) = if na == nb {
                (DuplicateKind::Exact, 1.0)
            } else {
                let sim = similarity_ratio(na, nb);
                if sim < opts.similarity_threshold {
                    continue;
                }
                (DuplicateKind::Near, sim)
            };
            let (keep, drop) = order_pair(a, i, b, j);
            let trim = compute_trim(&utterances[keep].text, &utterances[drop].text);
            out.push(DuplicatePair {
                keep,
                drop,
                kind,
                similarity,
                trim,
            });
        }
    }
    out
}

/// "overlapping timestamps" or "short temporal distance" (ADR 0012 §2).
fn temporally_close(a: &Utterance, b: &Utterance, max_gap_ms: u64) -> bool {
    let overlaps = a.start_ms < b.end_ms && b.start_ms < a.end_ms;
    if overlaps {
        return true;
    }
    let gap = if a.end_ms <= b.start_ms {
        b.start_ms - a.end_ms
    } else {
        a.start_ms - b.end_ms
    };
    gap <= max_gap_ms
}

fn order_pair(a: &Utterance, i: usize, b: &Utterance, j: usize) -> (usize, usize) {
    if a.start_ms <= b.start_ms {
        (i, j)
    } else {
        (j, i)
    }
}

fn compute_trim(keep_text: &str, drop_text: &str) -> TrimAction {
    contained_remainder(keep_text, drop_text).map_or(TrimAction::RemoveWhole, TrimAction::TrimTo)
}

const BOUNDARY_TRIM: &[char] = &[' ', ',', '.', ';', ':', '-', '\t', '\n'];

/// If `outer` case-insensitively starts or ends with `inner` (after
/// trimming surrounding whitespace) *and* has a genuine remainder beyond
/// that, returns the remainder (with adjoining boundary punctuation/space
/// stripped). `None` when `inner` isn't a clean prefix/suffix of `outer`,
/// when nothing is left after stripping, or when case-folding changed
/// `outer`'s character count (a handful of Unicode letters expand when
/// lowercased — skip rather than risk misaligned indexing on those).
fn contained_remainder(inner: &str, outer: &str) -> Option<String> {
    let inner_lower: Vec<char> = inner.trim().to_lowercase().chars().collect();
    let outer_trimmed: Vec<char> = outer.trim().chars().collect();
    let outer_lower: Vec<char> = outer.trim().to_lowercase().chars().collect();
    if inner_lower.is_empty()
        || outer_lower.len() <= inner_lower.len()
        || outer_lower.len() != outer_trimmed.len()
    {
        return None;
    }
    if outer_lower.starts_with(inner_lower.as_slice()) {
        let remainder: String = outer_trimmed[inner_lower.len()..].iter().collect();
        let trimmed = remainder.trim_start_matches(BOUNDARY_TRIM);
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if outer_lower.ends_with(inner_lower.as_slice()) {
        let cut = outer_lower.len() - inner_lower.len();
        let remainder: String = outer_trimmed[..cut].iter().collect();
        let trimmed = remainder.trim_end_matches(BOUNDARY_TRIM);
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    None
}

/// Case-insensitive, punctuation-stripped, whitespace-collapsed comparison
/// key. Deliberately aggressive — this is only used to compare candidate
/// duplicates, never written back anywhere.
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = false;
    for c in text.trim().chars() {
        if c.is_whitespace() {
            if !last_was_space && !out.is_empty() {
                out.push(' ');
                last_was_space = true;
            }
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            last_was_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

fn similarity_ratio(a: &str, b: &str) -> f64 {
    vd_text::similarity::similarity_ratio(a, b)
}
