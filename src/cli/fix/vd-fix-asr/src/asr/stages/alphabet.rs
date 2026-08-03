//! Stage 5 — Latin/Cyrillic homoglyph normalization (ADR 0010): `SРE` (Latin
//! S/E, Cyrillic Р) → `SPE`.
//!
//! Note on the ADR's own example: its prose shows `SРE → SRE`, but Cyrillic
//! `Р` (U+0420) is a true glyph-for-glyph homoglyph of Latin `P`, not `R` —
//! that reading doesn't correspond to any real confusable pair. Treated here
//! as a likely typo for `SРE → SPE`; this stage never maps `Р` to `R`.
//!
//! Only applied when every non-dominant-script letter in a token has a known
//! homoglyph in the dominant script ("only when confidence is extremely
//! high", per the ADR) — otherwise the token is left untouched.

use super::token::tokenize;
use super::RuleStage;
use crate::asr::rule::{Confidence, Rule, RuleCategory, RuleHit};
use crate::asr::stages::StageId;

/// Visually-identical Latin/Cyrillic letter pairs (case-sensitive; not every
/// letter has a lowercase-identical counterpart, e.g. Latin `b` vs Cyrillic
/// `в` are not confusable, so only `B`/`В` is listed).
const HOMOGLYPHS: &[(char, char)] = &[
    ('A', 'А'),
    ('a', 'а'),
    ('B', 'В'),
    ('E', 'Е'),
    ('e', 'е'),
    ('K', 'К'),
    ('k', 'к'),
    ('M', 'М'),
    ('H', 'Н'),
    ('O', 'О'),
    ('o', 'о'),
    ('P', 'Р'),
    ('p', 'р'),
    ('C', 'С'),
    ('c', 'с'),
    ('T', 'Т'),
    ('X', 'Х'),
    ('x', 'х'),
];

fn is_cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&c)
}

fn to_dominant(c: char, want_cyrillic: bool) -> Option<char> {
    HOMOGLYPHS.iter().find_map(|(latin, cyr)| {
        if want_cyrillic && *latin == c {
            Some(*cyr)
        } else if !want_cyrillic && *cyr == c {
            Some(*latin)
        } else {
            None
        }
    })
}

/// Normalizes a single token if it mixes scripts with a clear (non-tied)
/// majority and every minority-script letter is a known homoglyph.
fn normalize_token(word: &str) -> Option<String> {
    let (mut latin, mut cyr) = (0usize, 0usize);
    for c in word.chars() {
        if c.is_ascii_alphabetic() {
            latin += 1;
        } else if is_cyrillic(c) {
            cyr += 1;
        }
    }
    if latin == 0 || cyr == 0 || latin == cyr {
        return None;
    }
    let want_cyrillic = cyr > latin;
    let minority_is_cyrillic = !want_cyrillic;
    let mut out = String::with_capacity(word.len());
    let mut changed = false;
    for c in word.chars() {
        let in_minority = if minority_is_cyrillic {
            is_cyrillic(c)
        } else {
            c.is_ascii_alphabetic()
        };
        if in_minority {
            // Unresolvable minority letter: abstain on the whole token.
            out.push(to_dominant(c, want_cyrillic)?);
            changed = true;
        } else {
            out.push(c);
        }
    }
    changed.then_some(out)
}

struct HomoglyphNormalize;
impl Rule for HomoglyphNormalize {
    fn category(&self) -> RuleCategory {
        RuleCategory::Alphabet
    }

    fn apply(&self, input: &str) -> (String, Vec<RuleHit>) {
        let mut out = String::with_capacity(input.len());
        let mut hits = Vec::new();
        for tok in tokenize(input) {
            if tok.is_word {
                if let Some(normalized) = normalize_token(tok.text) {
                    hits.push(RuleHit {
                        category: RuleCategory::Alphabet,
                        confidence: Confidence::Certain,
                        rule_id: "alphabet:homoglyph",
                        span_id: None,
                        before: tok.text.to_string(),
                        after: normalized.clone(),
                    });
                    out.push_str(&normalized);
                    continue;
                }
            }
            out.push_str(tok.text);
        }
        (out, hits)
    }
}

pub fn stage() -> RuleStage {
    RuleStage::new(StageId::Alphabet, vec![Box::new(HomoglyphNormalize)])
}
