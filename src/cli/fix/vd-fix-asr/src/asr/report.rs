//! `--report` aggregation (ADR 0010).

use std::collections::BTreeMap;

use super::rule::{RuleCategory, RuleHit};
use super::stages::ConfidencePolicy;

const CATEGORIES: [RuleCategory; 7] = [
    RuleCategory::Spacing,
    RuleCategory::Punctuation,
    RuleCategory::Duplicate,
    RuleCategory::Merge,
    RuleCategory::Split,
    RuleCategory::Dictionary,
    RuleCategory::Alphabet,
];

/// Per-category counts of hits `policy` actually applied.
///
/// Plus `unsafe`: hits that were found but withheld (not just
/// `Confidence::Unsafe` ones — anything the active policy declined to
/// apply, e.g. `Likely` under `--strict`).
pub fn aggregate(hits: &[RuleHit], policy: ConfidencePolicy) -> BTreeMap<&'static str, u32> {
    let mut counts: BTreeMap<&'static str, u32> =
        CATEGORIES.iter().map(|c| (c.report_key(), 0)).collect();
    let mut withheld = 0u32;
    for hit in hits {
        if policy.allows(hit.confidence) {
            *counts.entry(hit.category.report_key()).or_insert(0) += 1;
        } else {
            withheld += 1;
        }
    }
    counts.insert("unsafe", withheld);
    counts
}
