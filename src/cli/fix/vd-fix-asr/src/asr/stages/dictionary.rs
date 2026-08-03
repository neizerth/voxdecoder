//! Stage 6 (static half) — curated ASR-mistake dictionary lookup, `Certain`
//! confidence. Backed by [`crate::asr::lang::resolve_dictionary`]'s layered
//! `builtin → pack → project → user` map.
//!
//! The context/neighbor fuzzy-matching half of Stage 6 needs `SpanContext`
//! (which words a word to fix is easier, given its neighbors), so it can't
//! fit this context-free `Stage`/`Pipeline` abstraction — it lives in
//! `asr/context_fuzzy.rs` and runs as a separate pass in `asr/fixer.rs`.
//! Both halves tag their hits `RuleCategory::Dictionary` so `--report`'s
//! `dictionary` counter covers the whole stage.

use std::collections::HashMap;
use std::sync::Arc;

use super::token::tokenize;
use super::RuleStage;
use crate::asr::rule::{Confidence, Rule, RuleCategory, RuleHit};
use crate::asr::stages::StageId;

/// Restores the original token's case onto a (possibly multi-word)
/// replacement. Compound fixes (containing a space) are never case-morphed.
pub fn restore_shape(original: &str, replacement: &str) -> String {
    if replacement.contains(' ') {
        return replacement.to_string();
    }
    if original.chars().all(char::is_uppercase) {
        return replacement.to_uppercase();
    }
    let mut chars = original.chars();
    let Some(first) = chars.next() else {
        return replacement.to_string();
    };
    if first.is_uppercase() && chars.all(|c| c.is_lowercase() || !c.is_alphabetic()) {
        let mut out = replacement.to_string();
        if let Some(r0) = out.chars().next() {
            out = r0.to_uppercase().collect::<String>() + &out[r0.len_utf8()..];
        }
        return out;
    }
    replacement.to_string()
}

struct StaticLookup(Arc<HashMap<String, String>>);

impl Rule for StaticLookup {
    fn category(&self) -> RuleCategory {
        RuleCategory::Dictionary
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        let mut out = String::with_capacity(input.len());
        let mut hits = Vec::new();
        for tok in tokenize(input) {
            if tok.is_word {
                let lower = tok.text.to_lowercase();
                if let Some(replacement) = self.0.get(&lower) {
                    let restored = restore_shape(tok.text, replacement);
                    hits.push(RuleHit {
                        category: RuleCategory::Dictionary,
                        confidence: Confidence::Certain,
                        rule_id: "dictionary:static",
                        span_id: None,
                        before: tok.text.to_string(),
                        after: restored.clone(),
                    });
                    out.push_str(&restored);
                    continue;
                }
            }
            out.push_str(tok.text);
        }
        (out, hits)
    }
}

// Always built from `lang::resolve_dictionary`'s std `HashMap` — no caller
// needs a custom hasher here.
#[allow(clippy::implicit_hasher)]
pub fn stage(map: Arc<HashMap<String, String>>) -> RuleStage {
    RuleStage::new(StageId::Dictionary, vec![Box::new(StaticLookup(map))])
}
