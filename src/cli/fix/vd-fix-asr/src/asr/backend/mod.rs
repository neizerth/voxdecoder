//! Private wording-repair backend (rules + context vocabulary).

use crate::context::SpanContext;
use crate::types::Language;

/// Apply recognition repairs. Must not translate, summarize, or invent unsupported words.
pub fn rewrite(text: &str, language: Language, ctx: SpanContext<'_>) -> String {
    let lang = match language {
        Language::En => Language::En,
        _ => Language::Ru, // ru / de / auto → ru-priority path for now
    };
    repair_tokens(text, lang, ctx)
}

fn repair_tokens(text: &str, language: Language, ctx: SpanContext<'_>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let (token, after) = next_token(rest);
        if token.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '+')
            && !token.is_empty()
        {
            out.push_str(&repair_one(token, language, ctx));
        } else {
            out.push_str(token);
        }
        rest = after;
    }
    out
}

fn next_token(s: &str) -> (&str, &str) {
    let mut chars = s.char_indices();
    let Some((_, first)) = chars.next() else {
        return ("", "");
    };
    let wordish = |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '+';
    if wordish(first) {
        for (i, c) in chars {
            if !wordish(c) {
                return (&s[..i], &s[i..]);
            }
        }
        (s, "")
    } else {
        for (i, c) in chars {
            if wordish(c) {
                return (&s[..i], &s[i..]);
            }
        }
        (s, "")
    }
}

fn repair_one(token: &str, language: Language, ctx: SpanContext<'_>) -> String {
    let lower = token.to_lowercase();

    // 1) Builtin ASR confusion lexicon (recognition, not canonical product names).
    if let Some(fixed) = builtin_map(language, &lower) {
        return restore_shape(token, fixed);
    }

    // 2) Multi-token compounds in lexicon (e.g. сейфтензорс → сейф тензорс) handled above.

    // 3) Context vocabulary: edit-distance ≤ 1 to a material token (supported by --context).
    if let Some(form) = closest_context_form(&lower, ctx) {
        if form.to_lowercase() != lower {
            return restore_shape(token, &form);
        }
    }

    // 4) Neighbors as weak vocabulary (same edit-distance rule).
    if let Some(form) = closest_neighbor_form(&lower, ctx) {
        if form.to_lowercase() != lower {
            return restore_shape(token, &form);
        }
    }

    token.to_string()
}

fn builtin_map(language: Language, lower: &str) -> Option<&'static str> {
    let table: &[(&str, &str)] = match language {
        Language::En => EN_LEXICON,
        _ => RU_LEXICON,
    };
    table
        .iter()
        .find(|(from, _)| *from == lower)
        .map(|(_, to)| *to)
}

/// Russian + English-insertion ASR confusions (phonetic), not project-canonical names.
const RU_LEXICON: &[(&str, &str)] = &[
    ("гитхап", "гитхаб"),
    ("гиттхаб", "гитхаб"),
    ("гитхапп", "гитхаб"),
    ("экшинс", "экшенс"),
    ("экшнс", "экшенс"),
    ("экшенз", "экшенс"),
    ("кубернетис", "кубернетес"),
    ("кубернетиз", "кубернетес"),
    ("сейфтензорс", "сейф тензорс"),
    ("сейфтензор", "сейф тензор"),
    ("рест-апи", "рест апи"),
    ("рестапи", "рест апи"),
    ("дотнет", "дот нет"),
];

const EN_LEXICON: &[(&str, &str)] = &[
    ("githap", "github"),
    ("githapp", "github"),
    ("gitub", "github"),
    ("actoins", "actions"),
    ("aciton", "action"),
    ("kubernetis", "kubernetes"), // phonetic; terms CLI may still canonicalize casing/brand
    ("safetensores", "safetensors"),
];

fn closest_context_form(lower: &str, ctx: SpanContext<'_>) -> Option<String> {
    if lower.chars().count() < 3 {
        return None;
    }
    let mut best: Option<(usize, String)> = None;
    for form in &ctx.materials.forms {
        let cand = form.to_lowercase();
        if cand == *lower {
            return Some(form.clone());
        }
        let dist = edit_distance(lower, &cand);
        if dist == 1 || (dist == 2 && lower.chars().count() >= 8) {
            match &best {
                None => best = Some((dist, form.clone())),
                Some((d, _)) if dist < *d => best = Some((dist, form.clone())),
                _ => {}
            }
        }
    }
    best.map(|(_, s)| s)
}

fn closest_neighbor_form(lower: &str, ctx: SpanContext<'_>) -> Option<String> {
    if lower.chars().count() < 4 {
        return None;
    }
    let mut best: Option<(usize, String)> = None;
    for neigh in ctx.neighbors_before.iter().chain(ctx.neighbors_after.iter()) {
        for token in neigh.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-')) {
            if token.is_empty() {
                continue;
            }
            let cand = token.to_lowercase();
            let dist = edit_distance(lower, &cand);
            if dist == 1 {
                match &best {
                    None => best = Some((dist, token.to_string())),
                    Some((d, _)) if dist < *d => best = Some((dist, token.to_string())),
                    _ => {}
                }
            }
        }
    }
    best.map(|(_, s)| s)
}

fn restore_shape(original: &str, replacement: &str) -> String {
    // If replacement has spaces (compound fix), don't try case morphing.
    if replacement.contains(' ') {
        return replacement.to_string();
    }
    if original.chars().all(char::is_uppercase) {
        return replacement.to_uppercase();
    }
    let mut chars = original.chars();
    let Some(first) = chars.next() else {
        return replacement.to_string();
    };
    if first.is_uppercase() && chars.all(|c| c.is_lowercase() || !c.is_alphabetic()) {
        let mut out = replacement.to_string();
        if let Some(r0) = out.chars().next() {
            out = r0.to_uppercase().collect::<String>() + &out[r0.len_utf8()..];
        }
        return out;
    }
    replacement.to_string()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n.abs_diff(m) > 2 {
        return 3;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1)
                .min(cur[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}
