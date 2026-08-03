//! Stage 3 — duplicate-token removal + filler-syllable collapse (ADR 0010).
//!
//! Adjacent whole-word duplicates and repeated-single-char filler runs are
//! `Certain` (structural, no ambiguity). Within-token self-doubling
//! (`каккак` → `как`) is `Likely` — short doubled real words exist, so it's
//! gated by `ConfidencePolicy` instead of always applied.

use super::token::tokenize;
use super::{ConfidencePolicy, Stage, StageId, StageOutcome};
use crate::asr::rule::{Confidence, RuleCategory, RuleHit};

fn hit(rule_id: &'static str, confidence: Confidence, before: String, after: String) -> RuleHit {
    RuleHit {
        category: RuleCategory::Duplicate,
        confidence,
        rule_id,
        span_id: None,
        before,
        after,
    }
}

/// `эти эти стандарты` → `эти стандарты`, `вот вот вот` → `вот`.
/// Case-insensitive; keeps the first occurrence, drops the rest along with
/// the single trailing space that separated them.
fn collapse_adjacent_word_duplicates(input: &str) -> (String, Vec<RuleHit>) {
    let toks = tokenize(input);
    let mut out = String::with_capacity(input.len());
    let mut hits = Vec::new();
    let mut last_word: Option<String> = None;
    let mut pending_trivial_space = false;
    for tok in &toks {
        if tok.is_word {
            let lower = tok.text.to_lowercase();
            if pending_trivial_space && last_word.as_deref() == Some(lower.as_str()) {
                let prev = last_word.as_deref().unwrap_or_default().to_string();
                out.pop();
                hits.push(hit(
                    "duplicate:adjacent-word",
                    Confidence::Certain,
                    format!("{prev} {}", tok.text),
                    prev,
                ));
                pending_trivial_space = false;
                continue;
            }
            out.push_str(tok.text);
            last_word = Some(lower);
            pending_trivial_space = false;
        } else {
            out.push_str(tok.text);
            pending_trivial_space = tok.text == " ";
            if tok.text != " " {
                last_word = None;
            }
        }
    }
    (out, hits)
}

/// `ииии` → `и` (meaningful alone, collapse fully); `ээээ` → `ээ` (not a
/// standalone word, keep a two-char hesitation marker).
fn collapse_filler_repeats(input: &str) -> (String, Vec<RuleHit>) {
    const KEEP_ONE: [char; 2] = ['и', 'i'];
    let toks = tokenize(input);
    let mut out = String::with_capacity(input.len());
    let mut hits = Vec::new();
    for tok in &toks {
        if tok.is_word {
            if let Some(collapsed) = filler_repeat_collapse(tok.text, &KEEP_ONE) {
                hits.push(hit(
                    "duplicate:filler-repeat",
                    Confidence::Certain,
                    tok.text.to_string(),
                    collapsed.clone(),
                ));
                out.push_str(&collapsed);
                continue;
            }
        }
        out.push_str(tok.text);
    }
    (out, hits)
}

fn filler_repeat_collapse(word: &str, keep_one: &[char]) -> Option<String> {
    let lower = word.to_lowercase();
    let mut chars = lower.chars();
    let first = chars.next()?;
    let count = 1 + chars.by_ref().filter(|c| *c == first).count();
    if count != lower.chars().count() || count < 3 {
        return None;
    }
    let keep = if keep_one.contains(&first) { 1 } else { 2 };
    Some(std::iter::repeat_n(first, keep).collect())
}

/// `каккак` → `как`: a word that is exactly two copies of the same
/// (case-insensitive) half concatenated. Requires a half length ≥ 3 so short
/// real words (`мама`, `папа`) never match.
fn self_duplicated_half(word: &str) -> Option<&str> {
    let n = word.chars().count();
    if n < 6 || n % 2 != 0 {
        return None;
    }
    let mid = word.char_indices().nth(n / 2)?.0;
    let (a, b) = word.split_at(mid);
    if a.eq_ignore_ascii_case(b) && !a.chars().all(|c| c == a.chars().next().unwrap_or(' ')) {
        Some(a)
    } else {
        None
    }
}

fn collapse_within_token_doubling(input: &str, policy: ConfidencePolicy) -> (String, Vec<RuleHit>) {
    let toks = tokenize(input);
    let mut out = String::with_capacity(input.len());
    let mut hits = Vec::new();
    for tok in &toks {
        if tok.is_word {
            if let Some(half) = self_duplicated_half(tok.text) {
                hits.push(hit(
                    "duplicate:within-token",
                    Confidence::Likely,
                    tok.text.to_string(),
                    half.to_string(),
                ));
                if policy.allows(Confidence::Likely) {
                    out.push_str(half);
                    continue;
                }
            }
        }
        out.push_str(tok.text);
    }
    (out, hits)
}

pub struct DuplicateStage;

impl Stage for DuplicateStage {
    fn id(&self) -> StageId {
        StageId::Duplicate
    }

    fn run(&self, input: &str, policy: &ConfidencePolicy) -> StageOutcome {
        let (text, mut hits) = collapse_filler_repeats(input);
        let (text, adjacent_hits) = collapse_adjacent_word_duplicates(&text);
        hits.extend(adjacent_hits);
        let (text, within_hits) = collapse_within_token_doubling(&text, *policy);
        hits.extend(within_hits);
        StageOutcome { text, hits }
    }
}

pub fn stage() -> DuplicateStage {
    DuplicateStage
}
