//! Staged deterministic pipeline (ADR 0010). Empty/identity today — stages
//! are added one at a time in follow-up PRs without changing this shape.

pub mod alphabet;
pub mod dictionary;
pub mod duplicate;
pub mod merge_split;
pub mod punctuation;
pub mod spacing;
pub mod token;

use super::rule::{Confidence, Rule, RuleHit};

/// Fixed pipeline order, mirrors the ADR's stage table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageId {
    Spacing,
    Punctuation,
    Duplicate,
    MergeSplit,
    Alphabet,
    Dictionary,
}

/// Which confidence levels get applied vs. only reported. Default (both
/// flags false) applies `Certain` + `Likely`, discards `Unsafe`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConfidencePolicy {
    /// `--strict`: apply `Certain` only.
    pub certain_only: bool,
    /// `--aggressive`: also apply `Unsafe`.
    pub allow_unsafe: bool,
}

impl ConfidencePolicy {
    /// Whether a hit at this confidence should be committed. Concrete stages
    /// call this to decide `outcome.text` while still recording every hit.
    pub const fn allows(self, confidence: Confidence) -> bool {
        match confidence {
            Confidence::Certain => true,
            Confidence::Likely => !self.certain_only,
            Confidence::Unsafe => self.allow_unsafe,
        }
    }
}

pub struct StageOutcome {
    pub text: String,
    /// Every hit found, including discarded ones (needed for `--report`'s
    /// `unsafe` counter).
    pub hits: Vec<RuleHit>,
}

/// One cleanup stage: a bundle of rules of the same `RuleCategory` that run
/// in sequence over the stage's input text.
pub trait Stage {
    fn id(&self) -> StageId;

    fn run(&self, input: &str, policy: &ConfidencePolicy) -> StageOutcome;
}

/// Ordered stage sequence. Stages are folded left-to-right: each stage sees
/// the previous stage's (possibly rewritten) text.
#[derive(Default)]
pub struct Pipeline {
    stages: Vec<Box<dyn Stage>>,
}

impl Pipeline {
    pub const fn new(stages: Vec<Box<dyn Stage>>) -> Self {
        Self { stages }
    }

    /// Runs every stage in order, threading text and accumulating hits.
    pub fn run(&self, input: &str, policy: &ConfidencePolicy) -> StageOutcome {
        let mut text = input.to_string();
        let mut hits = Vec::new();
        for stage in &self.stages {
            let outcome = stage.run(&text, policy);
            text = outcome.text;
            hits.extend(outcome.hits);
        }
        StageOutcome { text, hits }
    }
}

/// A stage that is just its rules, run in order, each seeing the previous
/// rule's output.
///
/// Rules registered here must only ever emit `Certain` hits — stages that
/// need to selectively keep/discard `Likely`/`Unsafe` proposals
/// (duplicate-token, merge/split, dictionary) implement `Stage` directly
/// instead of using this helper.
pub struct RuleStage {
    id: StageId,
    rules: Vec<Box<dyn Rule>>,
}

impl RuleStage {
    pub fn new(id: StageId, rules: Vec<Box<dyn Rule>>) -> Self {
        Self { id, rules }
    }
}

impl Stage for RuleStage {
    fn id(&self) -> StageId {
        self.id
    }

    fn run(&self, input: &str, _policy: &ConfidencePolicy) -> StageOutcome {
        let mut text = input.to_string();
        let mut hits = Vec::new();
        for rule in &self.rules {
            let (out, rule_hits) = rule.apply(&text);
            debug_assert!(
                rule_hits
                    .iter()
                    .all(|h| h.confidence == Confidence::Certain),
                "RuleStage rules must only emit Certain hits"
            );
            text = out;
            hits.extend(rule_hits);
        }
        StageOutcome { text, hits }
    }
}
