//! Deterministic rule primitives shared by every cleanup stage (ADR 0010).

use crate::types::SpanId;

/// How sure a rule is about a proposed change. `Unsafe` is never auto-applied
/// unless the caller opts in (`--aggressive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    Certain,
    Likely,
    Unsafe,
}

/// Which class of cleanup a rule belongs to (ADR 0010 "Rule engine" table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCategory {
    Spacing,
    Punctuation,
    Duplicate,
    Merge,
    Split,
    Dictionary,
    Alphabet,
}

impl RuleCategory {
    /// Stable lowercase key for `--report` JSON output.
    pub const fn report_key(self) -> &'static str {
        match self {
            Self::Spacing => "spacing",
            Self::Punctuation => "punctuation",
            Self::Duplicate => "duplicate",
            Self::Merge => "merge",
            Self::Split => "split",
            Self::Dictionary => "dictionary",
            Self::Alphabet => "alphabet",
        }
    }
}

/// One recorded (proposed or applied) change. `span_id` is filled in by the
/// pipeline driver once a rule runs against a real `TextSpan`; rules
/// themselves only see plain text and leave it as `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleHit {
    pub category: RuleCategory,
    pub confidence: Confidence,
    pub rule_id: &'static str,
    pub span_id: Option<SpanId>,
    pub before: String,
    pub after: String,
}

/// A single deterministic, stateless text transformation.
///
/// Implementations must always report every change they would make, even
/// ones the pipeline later discards for being `Unsafe` under the active
/// `ConfidencePolicy`.
pub trait Rule {
    fn category(&self) -> RuleCategory;

    /// Returns the rewritten text plus every hit found, applied or not.
    fn apply(&self, input: &str) -> (String, Vec<RuleHit>);
}
