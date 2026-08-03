//! Deterministic multi-pattern terminology matching (ADR 0013).
//!
//! Wraps `aho-corasick` behind a small `variant -> canonical` API: register
//! every known misspelling/alias once, then scan any text for every
//! occurrence in a single linear pass — instead of the naive "loop every
//! dictionary entry over the text" approach (`vd-fix-terms`'s lexicon
//! lookup does per-token `HashMap` lookups today, which is fine for exact
//! whole-token matches but doesn't generalize to substring/multi-word
//! terms the way an Aho-Corasick automaton does).

use std::collections::HashMap;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

#[derive(Debug, thiserror::Error)]
pub enum TermMatcherError {
    #[error("failed to build matcher: {0}")]
    Build(String),
}

/// One matched occurrence in a scanned text. `start`/`end` are byte offsets
/// into the scanned `&str`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermMatch {
    pub start: usize,
    pub end: usize,
    pub canonical: String,
}

/// Case-sensitive by default — see [`TermMatcher::new_ascii_case_insensitive`]
/// for the ASCII-only case-insensitive variant.
pub struct TermMatcher {
    ac: AhoCorasick,
    canonicals: Vec<String>,
}

impl TermMatcher {
    /// `entries`: `(variant, canonical)` pairs, exact byte match. Later
    /// entries for the same variant win — same "last wins" convention as
    /// every `vd-fix-*` dictionary layer.
    pub fn new(
        entries: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, TermMatcherError> {
        Self::build(entries, false)
    }

    /// ASCII-only case-insensitive matching (`GraphQL`/`graphql`/`GRAPHQL`
    /// all match a `GraphQL` pattern). Does **not** case-fold non-ASCII
    /// letters — Cyrillic terms need explicit variants for each casing you
    /// want matched, registered via [`TermMatcher::new`] instead.
    pub fn new_ascii_case_insensitive(
        entries: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, TermMatcherError> {
        Self::build(entries, true)
    }

    fn build(
        entries: impl IntoIterator<Item = (String, String)>,
        ascii_case_insensitive: bool,
    ) -> Result<Self, TermMatcherError> {
        let mut variants: Vec<String> = Vec::new();
        let mut canonicals: Vec<String> = Vec::new();
        let mut seen: HashMap<String, usize> = HashMap::new();
        for (variant, canonical) in entries {
            let key = if ascii_case_insensitive {
                variant.to_ascii_lowercase()
            } else {
                variant.clone()
            };
            if let Some(&idx) = seen.get(&key) {
                canonicals[idx] = canonical;
            } else {
                seen.insert(key, variants.len());
                variants.push(variant);
                canonicals.push(canonical);
            }
        }
        let ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(ascii_case_insensitive)
            .match_kind(MatchKind::LeftmostLongest)
            .build(&variants)
            .map_err(|e| TermMatcherError::Build(e.to_string()))?;
        Ok(Self { ac, canonicals })
    }

    /// Every non-overlapping match (leftmost-longest wins on overlap), in
    /// document order.
    pub fn find_all(&self, text: &str) -> Vec<TermMatch> {
        self.ac
            .find_iter(text)
            .map(|m| TermMatch {
                start: m.start(),
                end: m.end(),
                canonical: self.canonicals[m.pattern().as_usize()].clone(),
            })
            .collect()
    }

    /// Replaces every match with its canonical form; text outside matches
    /// is left untouched.
    pub fn replace_all(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for m in self.find_all(text) {
            out.push_str(&text[last..m.start]);
            out.push_str(&m.canonical);
            last = m.end;
        }
        out.push_str(&text[last..]);
        out
    }
}
