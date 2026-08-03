//! Stage 1 — whitespace normalization (ADR 0010): line endings, tabs, runs
//! of spaces. Character-level only; never touches paragraph structure.

use super::RuleStage;
use crate::asr::rule::{Confidence, Rule, RuleCategory, RuleHit};
use crate::asr::stages::StageId;

fn hit(rule_id: &'static str, before: String, after: String) -> RuleHit {
    RuleHit {
        category: RuleCategory::Spacing,
        confidence: Confidence::Certain,
        rule_id,
        span_id: None,
        before,
        after,
    }
}

struct NormalizeLineEndings;
impl Rule for NormalizeLineEndings {
    fn category(&self) -> RuleCategory {
        RuleCategory::Spacing
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        if !input.contains('\r') {
            return (input.to_string(), Vec::new());
        }
        let mut out = String::with_capacity(input.len());
        let mut hits = Vec::new();
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\r' {
                let had_lf = chars.peek() == Some(&'\n');
                if had_lf {
                    chars.next();
                }
                out.push('\n');
                hits.push(hit(
                    "spacing:line-ending",
                    if had_lf {
                        "\r\n".to_string()
                    } else {
                        "\r".to_string()
                    },
                    "\n".to_string(),
                ));
            } else {
                out.push(c);
            }
        }
        (out, hits)
    }
}

struct TabsToSpaces;
impl Rule for TabsToSpaces {
    fn category(&self) -> RuleCategory {
        RuleCategory::Spacing
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        if !input.contains('\t') {
            return (input.to_string(), Vec::new());
        }
        let mut hits = Vec::new();
        let out: String = input
            .chars()
            .map(|c| {
                if c == '\t' {
                    hits.push(hit("spacing:tab", "\t".to_string(), " ".to_string()));
                    ' '
                } else {
                    c
                }
            })
            .collect();
        (out, hits)
    }
}

struct CollapseSpaceRuns;
impl Rule for CollapseSpaceRuns {
    fn category(&self) -> RuleCategory {
        RuleCategory::Spacing
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        let mut out = String::with_capacity(input.len());
        let mut hits = Vec::new();
        let mut run = 0usize;
        let flush = |out: &mut String, hits: &mut Vec<RuleHit>, run: usize| {
            if run == 0 {
                return;
            }
            out.push(' ');
            if run > 1 {
                hits.push(hit(
                    "spacing:collapse-spaces",
                    " ".repeat(run),
                    " ".to_string(),
                ));
            }
        };
        for c in input.chars() {
            if c == ' ' {
                run += 1;
            } else {
                flush(&mut out, &mut hits, run);
                run = 0;
                out.push(c);
            }
        }
        flush(&mut out, &mut hits, run);
        (out, hits)
    }
}

pub fn stage() -> RuleStage {
    RuleStage::new(
        StageId::Spacing,
        vec![
            Box::new(NormalizeLineEndings),
            Box::new(TabsToSpaces),
            Box::new(CollapseSpaceRuns),
        ],
    )
}
