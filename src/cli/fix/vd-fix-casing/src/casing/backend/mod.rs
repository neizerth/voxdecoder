//! Private presentation backend (implementation detail).
//!
//! Today: linguistic rule engine for ASR-style `ru` / `en`.
//! Tomorrow: ONNX / Candle / ensemble — still behind this module only.

mod normalize;
mod restore;
mod tokens;

use crate::models::Lexicon;
use crate::types::Language;

use normalize::normalize;
use restore::restore_asr;

/// Apply presentation-only rewrite. Must not change word identity / translate / repair ASR.
pub fn rewrite(text: &str, language: Language, lexicon: &Lexicon) -> String {
    let lang = effective_language(language);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if looks_like_raw_asr(trimmed) {
        restore_asr(trimmed, lang, lexicon)
    } else {
        normalize(trimmed, lang)
    }
}

fn effective_language(language: Language) -> Language {
    match language {
        Language::Auto => Language::Ru,
        Language::De => Language::En, // until de ships: latin presentation path
        other => other,
    }
}

/// Raw ASR: little/no punctuation, mostly lowercase, no quotes/dashes yet.
///
/// Already-punctuated transcripts (GigaAM and similar) must take the
/// `normalize` path — `restore_asr` strips affix punctuation and re-predicts
/// only a small trail set, which destroys commas/periods/dashes.
fn looks_like_raw_asr(text: &str) -> bool {
    if text.chars().any(|ch| {
        matches!(
            ch,
            '"' | '«' | '»' | '“' | '”' | '„' | '—' | '–'
        )
    }) {
        return false;
    }

    let mut letters = 0usize;
    let mut upper = 0usize;
    let mut sentence_ends = 0usize;
    let mut other_punct = 0usize;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            letters += 1;
            if ch.is_uppercase() {
                upper += 1;
            }
        } else if matches!(ch, '.' | '!' | '?' | '…') {
            sentence_ends += 1;
        } else if matches!(ch, ',' | ';' | ':') {
            other_punct += 1;
        }
    }
    let punct = sentence_ends + other_punct;
    if letters < 8 {
        return sentence_ends == 0;
    }
    // Multiple sentences or several commas → already punctuated prose.
    if sentence_ends >= 2 || other_punct >= 2 {
        return false;
    }
    // Sparse punct + mostly lowercase → ASR restore path.
    punct * 20 < letters && upper * 8 < letters
}
