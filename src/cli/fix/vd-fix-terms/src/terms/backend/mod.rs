//! Private matcher / rewrite — longest lexicon hit wins, word-bounded.

use crate::lexicon::Lexicon;

pub fn rewrite(text: &str, lexicon: &Lexicon) -> String {
    if text.is_empty() || lexicon.is_empty() {
        return text.to_string();
    }

    let lower = text.to_lowercase();
    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();
    let n = chars.len();

    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;

    while i < n {
        let mut matched: Option<(usize, &str)> = None;
        for (variant, canonical) in lexicon.matchers() {
            let vchars: Vec<char> = variant.chars().collect();
            let vlen = vchars.len();
            if vlen == 0 || i + vlen > n {
                continue;
            }
            if lower_chars[i..i + vlen] != vchars[..] {
                continue;
            }
            if !is_boundary(&chars, i, vlen) {
                continue;
            }
            // Prefer first matcher (already sorted longest-first).
            matched = Some((vlen, canonical.as_str()));
            break;
        }

        if let Some((vlen, canonical)) = matched {
            // Skip rewrite if the surface already equals the canonical.
            let surface: String = chars[i..i + vlen].iter().collect();
            if surface == canonical {
                out.push_str(&surface);
            } else {
                out.push_str(canonical);
            }
            i += vlen;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    out
}

fn is_boundary(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !is_word_char(chars[start - 1]);
    let end = start + len;
    let after_ok = end >= chars.len() || !is_word_char(chars[end]);
    before_ok && after_ok
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
