//! Normalize already-punctuated (or mostly clean) text: quotes, dashes, spacing, casing.

use crate::types::Language;

pub fn normalize(text: &str, language: Language) -> String {
    let mut out = collapse_ws(text.trim());
    out = normalize_dashes(&out, language);
    out = normalize_quotes(&out, language);
    out = tidy_punct_spacing(&out);
    out = capitalize_sentences(&out);
    out = collapse_duplicate_periods(&out);
    ensure_terminal_punct(&out)
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

fn normalize_dashes(s: &str, language: Language) -> String {
    let dash = match language {
        Language::En => " – ",
        _ => " — ",
    };
    s.replace(" -- ", dash)
        .replace(" – ", dash)
        .replace(" — ", dash)
        .replace(" - ", dash)
}

fn normalize_quotes(s: &str, language: Language) -> String {
    match language {
        Language::En => to_ascii_double_quotes(s),
        _ => to_guillemets(s),
    }
}

fn to_guillemets(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut open = true;
    for ch in s.chars() {
        match ch {
            '"' | '“' | '”' => {
                out.push(if open { '«' } else { '»' });
                open = !open;
            }
            '«' => {
                out.push(ch);
                open = false;
            }
            '»' => {
                out.push(ch);
                open = true;
            }
            _ => out.push(ch),
        }
    }
    out
}

fn to_ascii_double_quotes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '«' | '»' | '“' | '”' | '"' => out.push('"'),
            _ => out.push(ch),
        }
    }
    out
}

fn tidy_punct_spacing(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if matches!(ch, ',' | '.' | ';' | ':' | '!' | '?') {
            while out.ends_with(' ') {
                out.pop();
            }
            out.push(ch);
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if next.is_whitespace() {
                    i += 1;
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if i < chars.len() && !matches!(chars[i], ',' | '.' | ';' | ':' | '!' | '?') {
                        out.push(' ');
                    }
                    continue;
                } else if next.is_alphanumeric() || matches!(next, '«' | '"' | '“') {
                    out.push(' ');
                }
            }
            i += 1;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

fn capitalize_sentences(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for ch in s.chars() {
        if capitalize_next && !ch.is_whitespace() {
            for c in ch.to_uppercase() {
                out.push(c);
            }
            capitalize_next = false;
        } else {
            out.push(ch);
        }
        if matches!(ch, '.' | '!' | '?' | '…') {
            capitalize_next = true;
        }
    }
    out
}

/// Collapse accidental `..` (and longer ASCII runs) without inventing new terminals.
/// `...` / longer → ellipsis `…`; bare `..` → single `.`.
fn collapse_duplicate_periods(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '.' {
            let mut n = 0usize;
            while i + n < chars.len() && chars[i + n] == '.' {
                n += 1;
            }
            if n >= 3 {
                out.push('…');
            } else {
                out.push('.');
            }
            i += n;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn ensure_terminal_punct(s: &str) -> String {
    if s.is_empty() || s.ends_with(['.', '!', '?', '…']) {
        s.to_string()
    } else {
        format!("{s}.")
    }
}
