//! Optional shipping lexicon slot — **empty by design**.
//!
//! Canonical terms come from `--terms` / project assets (`vd-assets`), not
//! from hardcoded tables in the binary. Residual brand/stack cleanup can use
//! meeting docs or a later AI pass.

use crate::types::Language;

use super::TermEntry;

/// Always empty — kept so `Lexicon::load(..., shipping: true)` stays valid
/// without embedding a word list in source.
pub fn entries(_language: Language) -> Vec<TermEntry> {
    Vec::new()
}
