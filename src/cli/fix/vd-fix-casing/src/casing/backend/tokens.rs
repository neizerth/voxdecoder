//! Whitespace tokenization that preserves word letter identity.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// Alphabetic/numeric/mark core — never rewritten to another lemma.
    pub core: String,
}

/// Split on whitespace; strip surrounding punctuation from each token into core + ignored affixes.
/// Affix punct is discarded in restore mode (re-predicted). Cores keep original characters.
pub fn words_from_asr(text: &str) -> Vec<Word> {
    text.split_whitespace()
        .filter_map(|tok| {
            let core = strip_punct_affixes(tok);
            if core.is_empty() {
                None
            } else {
                Some(Word { core })
            }
        })
        .collect()
}

fn strip_punct_affixes(tok: &str) -> String {
    let chars: Vec<char> = tok.chars().collect();
    let mut start = 0;
    let mut end = chars.len();
    while start < end && is_affix_punct(chars[start]) {
        start += 1;
    }
    while end > start && is_affix_punct(chars[end - 1]) {
        end -= 1;
    }
    chars[start..end].iter().collect()
}

fn is_affix_punct(ch: char) -> bool {
    matches!(
        ch,
        '.' | ','
            | '!'
            | '?'
            | ';'
            | ':'
            | '…'
            | '"'
            | '\''
            | '«'
            | '»'
            | '“'
            | '”'
            | '('
            | ')'
            | '['
            | ']'
            | '—'
            | '–'
            | '-'
    )
}

pub fn apply_case(core: &str, case: Case) -> String {
    match case {
        Case::Lower => core.to_lowercase(),
        Case::Capitalize => {
            let mut chars = core.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut out: String = first.to_uppercase().collect();
            out.extend(chars.flat_map(char::to_lowercase));
            out
        }
        Case::Upper => core.to_uppercase(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Lower,
    Capitalize,
    Upper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailPunct {
    None,
    Comma,
    Period,
    Question,
}

impl TrailPunct {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Comma => ",",
            Self::Period => ".",
            Self::Question => "?",
        }
    }
}

pub fn join_words(parts: &[(String, TrailPunct)]) -> String {
    let mut out = String::new();
    for (i, (word, punct)) in parts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(word);
        out.push_str(punct.as_str());
    }
    out
}
