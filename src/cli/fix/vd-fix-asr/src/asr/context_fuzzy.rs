//! Stage 6 (fuzzy half) — context/neighbor edit-distance correction. Needs
//! `SpanContext`, so it runs as its own pass in `fixer.rs` rather than
//! through the context-free `Stage`/`Pipeline` abstraction (see
//! `asr/stages/dictionary.rs` for the static-lookup half).
//!
//! Graded by edit distance: a single substitution/insertion is `Likely`; a
//! two-edit match on a longer token is `Unsafe`. Migrated unchanged from the
//! legacy lexicon backend, now confidence-graded instead of always applied.

use super::stages::dictionary::restore_shape;
use super::stages::token::tokenize;
use super::stages::ConfidencePolicy;
use crate::asr::rule::{Confidence, RuleCategory, RuleHit};
use crate::context::SpanContext;

pub fn apply(
    input: &str,
    ctx: SpanContext<'_>,
    policy: ConfidencePolicy,
) -> (String, Vec<RuleHit>) {
    let mut out = String::with_capacity(input.len());
    let mut hits = Vec::new();
    for tok in tokenize(input) {
        if tok.is_word {
            let lower = tok.text.to_lowercase();
            let found =
                closest_context_form(&lower, ctx).or_else(|| closest_neighbor_form(&lower, ctx));
            if let Some((form, dist)) = found {
                if form.to_lowercase() != lower {
                    let confidence = if dist <= 1 {
                        Confidence::Likely
                    } else {
                        Confidence::Unsafe
                    };
                    let restored = restore_shape(tok.text, &form);
                    hits.push(RuleHit {
                        category: RuleCategory::Dictionary,
                        confidence,
                        rule_id: "dictionary:context-fuzzy",
                        span_id: None,
                        before: tok.text.to_string(),
                        after: restored.clone(),
                    });
                    if policy.allows(confidence) {
                        out.push_str(&restored);
                        continue;
                    }
                }
            }
        }
        out.push_str(tok.text);
    }
    (out, hits)
}

fn closest_context_form(lower: &str, ctx: SpanContext<'_>) -> Option<(String, usize)> {
    if lower.chars().count() < 3 {
        return None;
    }
    let mut best: Option<(usize, String)> = None;
    // Prefer explicit forms, then vocabulary extracted from meeting docs / context.
    let candidates = ctx
        .materials
        .forms
        .iter()
        .chain(ctx.materials.vocabulary.iter());
    for form in candidates {
        let cand = form.to_lowercase();
        if cand == *lower {
            return Some((form.clone(), 0));
        }
        let dist = edit_distance(lower, &cand);
        if !plausible_asr_edit(lower, &cand, dist) {
            continue;
        }
        match &best {
            None => best = Some((dist, form.clone())),
            Some((d, _)) if dist < *d => best = Some((dist, form.clone())),
            _ => {}
        }
    }
    best.map(|(d, s)| (s, d))
}

fn closest_neighbor_form(lower: &str, ctx: SpanContext<'_>) -> Option<(String, usize)> {
    if lower.chars().count() < 4 {
        return None;
    }
    let mut best: Option<(usize, String)> = None;
    for neigh in ctx
        .neighbors_before
        .iter()
        .chain(ctx.neighbors_after.iter())
    {
        for token in neigh.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-')) {
            if token.is_empty() {
                continue;
            }
            let cand = token.to_lowercase();
            let dist = edit_distance(lower, &cand);
            if !plausible_asr_edit(lower, &cand, dist) {
                continue;
            }
            match &best {
                None => best = Some((dist, token.to_string())),
                Some((d, _)) if dist < *d => best = Some((dist, token.to_string())),
                _ => {}
            }
        }
    }
    best.map(|(d, s)| (s, d))
}

/// Accept near-miss ASR repairs; reject length-decreasing edits that strip
/// Russian case endings (e.g. neighbor `друг` must not rewrite `друга`).
fn plausible_asr_edit(from: &str, to: &str, dist: usize) -> bool {
    if dist == 0 {
        return true;
    }
    let n = from.chars().count();
    let m = to.chars().count();
    if dist == 1 {
        // Same length or longer always OK. One-char shortening only for longer
        // tokens (extra vowel / typo) — short tokens keep the case-ending guard.
        return m >= n || (n >= 8 && m + 1 >= n);
    }
    if dist == 2 && n >= 8 {
        return m >= n || m + 1 >= n;
    }
    false
}

fn edit_distance(a: &str, b: &str) -> usize {
    // Early-exit optimization: skip computing if length difference > 2
    // (plausible_asr_edit filters these anyway; return 3 as a "clearly too far" marker)
    let n = a.chars().count();
    let m = b.chars().count();
    if n.abs_diff(m) > 2 {
        return 3;
    }
    vd_text::similarity::edit_distance(a, b)
}
