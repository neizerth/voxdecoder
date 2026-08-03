//! Stage 4 — dictionary-assisted merge/split (ADR 0010): `этотоже` → `это
//! тоже` (split), `дата сет` → `датасет` (merge).
//!
//! Curated static tables only, `Certain` confidence — the same trust level
//! as the legacy lexicon backend's hardcoded substitutions. Fuzzy /
//! context-driven correction stays in Stage 6 (dictionary). Full builtin →
//! pack → project → user dictionary layering lands with Stage 6 in a later
//! PR; these tables are placeholders for that same mechanism.

use super::token::tokenize;
use super::RuleStage;
use crate::asr::rule::{Confidence, Rule, RuleCategory, RuleHit};
use crate::asr::stages::StageId;

const SPLIT_TABLE: &[(&str, &str)] = &[("этотоже", "это тоже")];

const MERGE_TABLE: &[(&str, &str, &str)] = &[("дата", "сет", "датасет")];

fn restore_leading_case(original: &str, replacement: &str) -> String {
    let Some(first_orig) = original.chars().next() else {
        return replacement.to_string();
    };
    if first_orig.is_uppercase() {
        let mut chars = replacement.chars();
        if let Some(first) = chars.next() {
            return first.to_uppercase().collect::<String>() + chars.as_str();
        }
    }
    replacement.to_string()
}

fn hit(category: RuleCategory, rule_id: &'static str, before: String, after: String) -> RuleHit {
    RuleHit {
        category,
        confidence: Confidence::Certain,
        rule_id,
        span_id: None,
        before,
        after,
    }
}

fn apply_split(input: &str) -> (String, Vec<RuleHit>) {
    let mut out = String::with_capacity(input.len());
    let mut hits = Vec::new();
    for tok in tokenize(input) {
        if tok.is_word {
            let lower = tok.text.to_lowercase();
            if let Some((_, split)) = SPLIT_TABLE.iter().find(|(k, _)| *k == lower) {
                let restored = restore_leading_case(tok.text, split);
                hits.push(hit(
                    RuleCategory::Split,
                    "merge_split:split",
                    tok.text.to_string(),
                    restored.clone(),
                ));
                out.push_str(&restored);
                continue;
            }
        }
        out.push_str(tok.text);
    }
    (out, hits)
}

fn apply_merge(input: &str) -> (String, Vec<RuleHit>) {
    let mut out = String::with_capacity(input.len());
    let mut hits = Vec::new();
    // (lowercase text, original text, byte offset in `out` where it starts)
    let mut last_word: Option<(String, String, usize)> = None;
    let mut pending_trivial_space = false;
    for tok in tokenize(input) {
        if tok.is_word {
            let lower = tok.text.to_lowercase();
            if pending_trivial_space {
                if let Some((prev_lower, prev_orig, start)) = &last_word {
                    if let Some((_, _, merged)) = MERGE_TABLE
                        .iter()
                        .find(|(a, b, _)| *a == prev_lower.as_str() && *b == lower.as_str())
                    {
                        let restored = restore_leading_case(prev_orig, merged);
                        out.truncate(*start);
                        hits.push(hit(
                            RuleCategory::Merge,
                            "merge_split:merge",
                            format!("{prev_orig} {}", tok.text),
                            restored.clone(),
                        ));
                        let merge_start = *start;
                        out.push_str(&restored);
                        last_word = Some((restored.to_lowercase(), restored, merge_start));
                        pending_trivial_space = false;
                        continue;
                    }
                }
            }
            let start = out.len();
            out.push_str(tok.text);
            last_word = Some((lower, tok.text.to_string(), start));
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

struct SplitRule;
impl Rule for SplitRule {
    fn category(&self) -> RuleCategory {
        RuleCategory::Split
    }
    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        apply_split(input)
    }
}

struct MergeRule;
impl Rule for MergeRule {
    fn category(&self) -> RuleCategory {
        RuleCategory::Merge
    }
    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        apply_merge(input)
    }
}

pub fn stage() -> RuleStage {
    // Split first: a merged-then-split round trip never happens with today's
    // tiny curated tables, but running split before merge keeps the pipeline
    // deterministic if that ever changes.
    RuleStage::new(
        StageId::MergeSplit,
        vec![Box::new(SplitRule), Box::new(MergeRule)],
    )
}
