//! Sentence splitting and discourse cue matching.

use crate::models::Lexicon;

/// Split prose into sentences without changing lexical tokens.
pub fn split_sentences(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut start = 0usize;
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();
    let mut i = 0usize;

    while i < len {
        let ch = chars[i];
        let is_end = matches!(ch, '.' | '!' | '?' | '…')
            || (ch == '.' && i + 1 < len && chars[i + 1] == '.');
        if is_end {
            // consume ….
            while i + 1 < len && matches!(chars[i + 1], '.' | '!' | '?' | '…') {
                i += 1;
            }
            let end = i + 1;
            // include trailing quotes/brackets
            let mut j = end;
            while j < len && matches!(chars[j], '"' | '\'' | '»' | '”' | ')' | ']') {
                j += 1;
            }
            let sentence: String = chars[start..j].iter().collect();
            let s = sentence.trim();
            if !s.is_empty() {
                out.push(s.to_string());
            }
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }
            start = j;
            i = j;
            continue;
        }
        i += 1;
    }

    if start < len {
        let sentence: String = chars[start..].iter().collect();
        let s = sentence.trim();
        if !s.is_empty() {
            out.push(s.to_string());
        }
    }

    if out.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

pub fn starts_with_cue(sentence: &str, cues: &[String]) -> bool {
    let lower = sentence.to_lowercase();
    let trimmed = lower.trim_start();
    for cue in cues {
        let c = cue.to_lowercase();
        if trimmed == c || trimmed.starts_with(&format!("{c} ")) || trimmed.starts_with(&format!("{c},"))
            || trimmed.starts_with(&format!("{c}."))
        {
            return true;
        }
    }
    false
}

pub fn discourse_break(sentence: &str, lexicon: &Lexicon) -> bool {
    starts_with_cue(sentence, &lexicon.discourse)
}

pub fn soft_break(sentence: &str, lexicon: &Lexicon) -> bool {
    starts_with_cue(sentence, &lexicon.soft_break)
}

/// Lexical fingerprint: alphanumeric tokens only (for guarantee tests).
pub fn lexical_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '\'' || ch == '’' || ch == '-' {
            cur.push(ch);
        } else if !cur.is_empty() {
            tokens.push(cur.to_lowercase());
            cur.clear();
        }
    }
    if !cur.is_empty() {
        tokens.push(cur.to_lowercase());
    }
    tokens
}
