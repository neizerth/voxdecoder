//! Text similarity (ADR 0013).
//!
//! Thin wrapper over `strsim` so `vd-fix-*` crates share one edit-distance
//! implementation instead of each hand-rolling its own — `vd-fix-asr`'s
//! `context_fuzzy` module and `vd-fix-overlap`'s `overlap::detect` module
//! both carry their own private, byte-for-byte-identical Levenshtein
//! implementation today. Migrating those to call this crate is a natural
//! follow-up, not done as part of adding this crate.

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
