//! Text similarity (ADR 0013).
//!
//! Thin wrapper over `strsim` so `vd-fix-*` crates share one edit-distance
//! implementation instead of each hand-rolling its own — `vd-fix-asr`'s
//! `context_fuzzy` module and `vd-fix-overlap`'s `overlap::detect` module
//! both carry their own private, byte-for-byte-identical Levenshtein
//! implementation today. Migrating those to call this crate is a natural
//! follow-up, not done as part of adding this crate.

use std::collections::HashSet;

/// Levenshtein edit distance, Unicode-scalar-aware (same granularity every
/// hand-rolled version in this workspace already uses — not grapheme
/// clusters).
pub fn edit_distance(a: &str, b: &str) -> usize {
    strsim::levenshtein(a, b)
}

/// Normalized similarity in `[0.0, 1.0]`: `1.0` = identical, `0.0` =
/// completely dissimilar (edit distance equals the longer string's length).
/// `1.0` for two empty strings.
pub fn similarity_ratio(a: &str, b: &str) -> f64 {
    strsim::normalized_levenshtein(a, b)
}

/// Token Jaccard over whitespace-split tokens (already-normalized text).
pub fn token_jaccard(a: &str, b: &str) -> f64 {
    let ta: HashSet<&str> = a.split_whitespace().collect();
    let tb: HashSet<&str> = b.split_whitespace().collect();
    if ta.is_empty() && tb.is_empty() {
        return 1.0;
    }
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    let inter = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// Fraction of the shorter string's tokens that appear in the longer
/// (ASR bleed often appends a tail on one track).
pub fn token_coverage(a: &str, b: &str) -> f64 {
    let ta: Vec<&str> = a.split_whitespace().collect();
    let tb: Vec<&str> = b.split_whitespace().collect();
    if ta.is_empty() && tb.is_empty() {
        return 1.0;
    }
    let (short, long) = if ta.len() <= tb.len() {
        (&ta[..], &tb[..])
    } else {
        (&tb[..], &ta[..])
    };
    if short.is_empty() {
        return 0.0;
    }
    let long_set: HashSet<&str> = long.iter().copied().collect();
    let hit = short.iter().filter(|t| long_set.contains(*t)).count() as f64;
    hit / short.len() as f64
}

/// Near-duplicate score for ASR bleed / mix residual (ADR 0012 / 0016).
///
/// Pure Levenshtein under-scores long near-copies with a length gap
/// (difflib ≈0.87 while lev ≈0.76). Combine lev, bigram Dice, token Jaccard,
/// and short→long token coverage — take the max.
///
/// Token coverage alone is ignored for tiny shorts (`ок`, `да`) — too easy
/// to false-positive inside an overlapping window.
pub fn asr_near_duplicate_ratio(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let lev = similarity_ratio(a, b);
    let dice = strsim::sorensen_dice(a, b);
    let jaccard = token_jaccard(a, b);
    let short_tokens = a
        .split_whitespace()
        .count()
        .min(b.split_whitespace().count());
    let coverage = if short_tokens >= 8 {
        token_coverage(a, b)
    } else {
        0.0
    };
    lev.max(dice).max(jaccard).max(coverage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_bleed_with_length_gap_scores_high() {
        // Real 2026-07-31 meeting pair (normalized): lev alone ~0.76.
        let a = "продукта зависят и я с тобой согласен это круто когда есть девопсы которые могут всем рулить потому что ну типа ты можешь быть супер спецом во всех областях но ты будешь терять чисто вот в глубине";
        let b = "продукта зависят я с тобой согласен это круто когда есть девопсы которые могут всем рулить потому что ну типа ты можешь быть супер спецом во всех областях но ты будешь терять чисто вот в глубине ну вот вот я как раз об этом и хотел донести ну и отсюда";
        let lev = similarity_ratio(a, b);
        let near = asr_near_duplicate_ratio(a, b);
        assert!(lev < 0.80, "lev alone should stay below old threshold, got {lev}");
        assert!(
            near >= 0.80,
            "combined near-dup must catch ASR bleed, got {near}"
        );
    }
}
