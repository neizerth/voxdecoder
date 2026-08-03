//! Stage 2 — punctuation normalization (ADR 0010): duplicate-dot ellipsis,
//! duplicate punctuation marks, space-before-punctuation.

use super::RuleStage;
use crate::asr::rule::{Confidence, Rule, RuleCategory, RuleHit};
use crate::asr::stages::StageId;

const DUPLICATE_MARKS: [char; 5] = ['!', '?', ',', ';', ':'];
const SPACE_GUARDED_MARKS: [char; 6] = [',', '.', '!', '?', ':', ';'];

fn hit(rule_id: &'static str, before: String, after: String) -> RuleHit {
    RuleHit {
        category: RuleCategory::Punctuation,
        confidence: Confidence::Certain,
        rule_id,
        span_id: None,
        before,
        after,
    }
}

/// Runs of 3+ `.` collapse to a single ellipsis: `....` → `…`.
struct CollapseEllipsis;
impl Rule for CollapseEllipsis {
    fn category(&self) -> RuleCategory {
        RuleCategory::Punctuation
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        let chars: Vec<char> = input.chars().collect();
        let mut out = String::with_capacity(input.len());
        let mut hits = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '.' {
                let start = i;
                while i < chars.len() && chars[i] == '.' {
                    i += 1;
                }
                let run_len = i - start;
                if run_len >= 3 {
                    out.push('…');
                    hits.push(hit(
                        "punctuation:ellipsis",
                        ".".repeat(run_len),
                        "…".to_string(),
                    ));
                } else {
                    out.extend(std::iter::repeat_n('.', run_len));
                }
            } else {
                out.push(chars[i]);
                i += 1;
            }
        }
        (out, hits)
    }
}

/// Runs of 2+ identical marks (from `!?,;:`) collapse to one: `!!!` → `!`.
struct CollapseDuplicateMarks;
impl Rule for CollapseDuplicateMarks {
    fn category(&self) -> RuleCategory {
        RuleCategory::Punctuation
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        let chars: Vec<char> = input.chars().collect();
        let mut out = String::with_capacity(input.len());
        let mut hits = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if DUPLICATE_MARKS.contains(&c) {
                let start = i;
                while i < chars.len() && chars[i] == c {
                    i += 1;
                }
                let run_len = i - start;
                out.push(c);
                if run_len > 1 {
                    hits.push(hit(
                        "punctuation:duplicate-mark",
                        c.to_string().repeat(run_len),
                        c.to_string(),
                    ));
                }
            } else {
                out.push(c);
                i += 1;
            }
        }
        (out, hits)
    }
}

/// Drops space(s) directly before a punctuation mark: `Да , конечно` →
/// `Да, конечно`.
struct RemoveSpaceBeforeMark;
impl Rule for RemoveSpaceBeforeMark {
    fn category(&self) -> RuleCategory {
        RuleCategory::Punctuation
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        let chars: Vec<char> = input.chars().collect();
        let mut out = String::with_capacity(input.len());
        let mut hits = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == ' ' {
                let start = i;
                let mut j = i;
                while j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                if j < chars.len() && SPACE_GUARDED_MARKS.contains(&chars[j]) {
                    hits.push(hit(
                        "punctuation:space-before-mark",
                        format!("{}{}", " ".repeat(j - start), chars[j]),
                        chars[j].to_string(),
                    ));
                    i = j;
                    continue;
                }
                out.extend(&chars[start..j]);
                i = j;
                continue;
            }
            out.push(chars[i]);
            i += 1;
        }
        (out, hits)
    }
}

pub fn stage() -> RuleStage {
    RuleStage::new(
        StageId::Punctuation,
        vec![
            Box::new(CollapseEllipsis),
            Box::new(CollapseDuplicateMarks),
            Box::new(RemoveSpaceBeforeMark),
        ],
    )
}
