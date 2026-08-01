//! Restore punctuation + casing for raw ASR transcripts without changing word identity.

use super::normalize::normalize;
use super::tokens::{apply_case, join_words, words_from_asr, Case, TrailPunct, Word};
use crate::models::Lexicon;
use crate::types::Language;

const MAX_SENTENCE_WORDS: usize = 18;

pub fn restore_asr(text: &str, language: Language, lexicon: &Lexicon) -> String {
    let words = words_from_asr(text);
    if words.is_empty() {
        return String::new();
    }

    let restored = restore_lang(&words, language, lexicon);
    normalize(&restored, language)
}

fn restore_lang(words: &[Word], language: Language, lexicon: &Lexicon) -> String {
    let lowers: Vec<String> = words.iter().map(|w| w.core.to_lowercase()).collect();
    let n = lowers.len();
    let mut trail = vec![TrailPunct::None; n];
    let mut cases = vec![Case::Lower; n];

    for (i, w) in lowers.iter().enumerate() {
        if is_discourse(w, lexicon) && i + 1 < n {
            trail[i] = TrailPunct::Comma;
        }
    }

    let mut since_break = 0usize;
    for i in 0..n {
        since_break += 1;
        let soft = is_soft_break(&lowers[i], lexicon);
        if i > 0
            && soft
            && since_break >= 6
            && trail[i - 1] == TrailPunct::None
            && since_break >= MAX_SENTENCE_WORDS / 2
        {
            trail[i - 1] = TrailPunct::Period;
            since_break = 1;
        } else if since_break >= MAX_SENTENCE_WORDS && i + 1 < n && trail[i] == TrailPunct::None {
            trail[i] = TrailPunct::Period;
            since_break = 0;
        }
    }

    apply_questions(&lowers, &mut trail, lexicon);

    if n > 0 && matches!(trail[n - 1], TrailPunct::None | TrailPunct::Comma) {
        trail[n - 1] = TrailPunct::Period;
    }

    cases[0] = Case::Capitalize;
    for i in 0..n.saturating_sub(1) {
        if matches!(trail[i], TrailPunct::Period | TrailPunct::Question) {
            cases[i + 1] = Case::Capitalize;
        }
    }

    if language == Language::En {
        for (i, w) in lowers.iter().enumerate() {
            if w == "i" {
                cases[i] = Case::Upper;
            }
        }
    }

    let parts: Vec<(String, TrailPunct)> = words
        .iter()
        .enumerate()
        .map(|(i, w)| (apply_case(&w.core, cases[i]), trail[i]))
        .collect();
    join_words(&parts)
}

fn apply_questions(lowers: &[String], trail: &mut [TrailPunct], lexicon: &Lexicon) {
    let n = lowers.len();
    if n == 0 {
        return;
    }

    let mut start = 0usize;
    while start < n {
        let mut end = start;
        while end < n {
            let is_end =
                end + 1 == n || matches!(trail[end], TrailPunct::Period | TrailPunct::Question);
            if is_end {
                break;
            }
            end += 1;
        }
        let last = end.min(n - 1);
        let slice = &lowers[start..=last];
        if is_question_span(slice, lexicon) {
            trail[last] = TrailPunct::Question;
        }
        start = last + 1;
    }
}

fn is_question_span(words: &[String], lexicon: &Lexicon) -> bool {
    if words.is_empty() {
        return false;
    }
    if lexicon.question_start.iter().any(|w| w == &words[0]) {
        return true;
    }
    words
        .iter()
        .any(|w| lexicon.question_particles.iter().any(|p| p == w))
}

fn is_discourse(w: &str, lexicon: &Lexicon) -> bool {
    lexicon.discourse.iter().any(|d| d == w)
}

fn is_soft_break(w: &str, lexicon: &Lexicon) -> bool {
    lexicon.soft_break.iter().any(|d| d == w)
}
