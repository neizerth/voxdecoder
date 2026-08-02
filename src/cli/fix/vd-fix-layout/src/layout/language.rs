//! Language resolution (`auto`) and detection.

use crate::types::Language;

/// Resolve `auto` → shipping pack language.
///
/// Order: artifact language (caller) → TimeMap language (future) → detect → config fallback.
pub fn resolve_language(
    requested: Language,
    sample_text: &str,
    config_fallback: Option<Language>,
) -> Language {
    match requested {
        Language::Ru | Language::En | Language::De => requested,
        Language::Auto => {
            if let Some(detected) = detect_language(sample_text) {
                return detected;
            }
            match config_fallback {
                Some(lang @ (Language::Ru | Language::En)) => lang,
                Some(Language::De) => Language::En,
                Some(Language::Auto) | None => Language::Ru,
            }
        }
    }
}

/// Heuristic: Cyrillic vs Latin letter counts → `ru` / `en`.
pub fn detect_language(text: &str) -> Option<Language> {
    let mut cyr = 0u32;
    let mut lat = 0u32;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            if ('\u{0400}'..='\u{04FF}').contains(&ch) {
                cyr += 1;
            } else if ch.is_ascii_alphabetic() {
                lat += 1;
            }
        }
    }
    let total = cyr + lat;
    if total < 8 {
        return None;
    }
    if cyr * 2 >= lat {
        Some(Language::Ru)
    } else {
        Some(Language::En)
    }
}
