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

/// Optional diarize timeline hint (ADR 0016 prefer-active speaker).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineHint {
    pub speaker: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

/// Detection thresholds (ADR 0012 §2: "Corrections require high
/// confidence. If uncertain: preserve both copies." — both knobs default
/// conservative).
#[derive(Debug, Clone, PartialEq)]
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
    /// When set, prefer keeping the utterance whose speaker matches the
    /// timeline-dominant speaker in the pair's time window (ADR 0016).
    pub timeline: Vec<TimelineHint>,
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self {
            // 0.80 catches same-window bleed with a truncated tail (meeting
            // 2026-07-31 style) while staying high-confidence for gap pairs.
            similarity_threshold: 0.80,
            max_gap_ms: 500,
            timeline: Vec::new(),
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
/// `keep` / `drop` are input indices — prefer timeline-active speaker when
/// hints match, else earlier start (ADR 0012 / 0016). This module only
/// *recommends*; it never mutates anything.
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
                let sim = pair_similarity(na, nb);
                if sim < opts.similarity_threshold {
                    continue;
                }
                (DuplicateKind::Near, sim)
            };
            let (keep, drop) = order_pair(a, i, b, j, &opts.timeline);
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
    if ranges_overlap(a, b) {
        return true;
    }
    let gap = if a.end_ms <= b.start_ms {
        b.start_ms - a.end_ms
    } else {
        a.start_ms - b.end_ms
    };
    gap <= max_gap_ms
}

fn ranges_overlap(a: &Utterance, b: &Utterance) -> bool {
    a.start_ms < b.end_ms && b.start_ms < a.end_ms
}

/// Prefer containment / shared-prefix signal when one track's ASR is a
/// truncated echo of the other (common bleed pattern).
fn pair_similarity(a: &str, b: &str) -> f64 {
    let base = vd_text::similarity::asr_near_duplicate_ratio(a, b);
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if short.is_empty() {
        return base;
    }
    // Prefix/suffix containment only — mid-string `contains` is too aggressive
    // for strict threshold tests and unrelated shared phrases.
    if long.starts_with(short) || long.ends_with(short) {
        let coverage = short.len() as f64 / long.len() as f64;
        return base.max(coverage);
    }
    // Same-window bleed with a mid-string micro-edit ("не плохо, не" vs
    // "не плохо и не"): full containment fails and Levenshtein dips below
    // 0.80, but a long shared prefix of the shorter turn is still a
    // high-confidence echo. Force into the near-dup band when ≥65% of the
    // shorter normalized text matches as a prefix.
    let lcp = longest_common_prefix_bytes(a, b);
    let lcp_short = lcp as f64 / short.len() as f64;
    if lcp_short >= 0.65 {
        return base.max(0.80_f64.max(lcp_short));
    }
    base
}

fn longest_common_prefix_bytes(a: &str, b: &str) -> usize {
    a.bytes()
        .zip(b.bytes())
        .take_while(|(x, y)| x == y)
        .count()
}

fn order_pair(
    a: &Utterance,
    i: usize,
    b: &Utterance,
    j: usize,
    timeline: &[TimelineHint],
) -> (usize, usize) {
    let win_start = a.start_ms.max(b.start_ms);
    let win_end = a.end_ms.min(b.end_ms);
    let window = if win_start < win_end {
        (win_start, win_end)
    } else {
        (a.start_ms.min(b.start_ms), a.end_ms.max(b.end_ms))
    };
    if let Some(preferred) = dominant_timeline_speaker(timeline, window.0, window.1) {
        let a_match = speakers_match(&a.speaker, &preferred);
        let b_match = speakers_match(&b.speaker, &preferred);
        if a_match && !b_match {
            return (i, j);
        }
        if b_match && !a_match {
            return (j, i);
        }
    }
    if a.start_ms <= b.start_ms {
        (i, j)
    } else {
        (j, i)
    }
}

fn speakers_match(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn dominant_timeline_speaker(
    timeline: &[TimelineHint],
    start_ms: u64,
    end_ms: u64,
) -> Option<String> {
    if timeline.is_empty() || start_ms >= end_ms {
        return None;
    }
    let mut best: Option<(String, u64)> = None;
    for hint in timeline {
        let overlap_start = hint.start_ms.max(start_ms);
        let overlap_end = hint.end_ms.min(end_ms);
        if overlap_start >= overlap_end {
            continue;
        }
        let dur = overlap_end - overlap_start;
        match &best {
            None => best = Some((hint.speaker.clone(), dur)),
            Some((_, best_dur)) if dur > *best_dur => {
                best = Some((hint.speaker.clone(), dur));
            }
            _ => {}
        }
    }
    best.map(|(s, _)| s)
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

/// Public normalize for merge subtract / tests (same key as duplicate detect).
pub fn normalize_for_compare(text: &str) -> String {
    normalize(text)
}
