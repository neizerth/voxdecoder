//! Deterministic disfluency rules (ADR 0012 §1).
//!
//! No regex dependency — a small hand-rolled word/separator tokenizer, in the
//! same spirit as `vd-fix-asr`'s `backend::next_token`. Everything here is a
//! pure text transform: `fix_text` is the only entry point other modules need.

use crate::types::{FixResult, Language};

/// How aggressively `vd-fix-disfluency` rewrites text. Default: `Light`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Mode {
    /// No changes at all.
    Off,
    /// Remove isolated fillers; collapse (not delete) repeated filler runs;
    /// apply the empty-hesitation composite cleanup. Never touches false starts.
    #[default]
    Light,
    /// Everything in `Light`, plus: filler runs are removed entirely (no
    /// residue) and false starts are collapsed.
    Normal,
    /// Same rule set as `Normal` for this scaffold — reserved for future,
    /// riskier transforms (see `STRUCTURE.md` non-goals).
    Aggressive,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "off" => Some(Self::Off),
            "light" => Some(Self::Light),
            "normal" => Some(Self::Normal),
            "aggressive" => Some(Self::Aggressive),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Light => "light",
            Self::Normal => "normal",
            Self::Aggressive => "aggressive",
        }
    }

    pub fn allowed() -> &'static [&'static str] {
        &["off", "light", "normal", "aggressive"]
    }
}

/// Filler syllables — removed entirely, all modes except `off` (ADR 0012 §1).
const FILLERS_RU: &[&str] = &["эээ", "ммм", "эм"];
const FILLERS_EN: &[&str] = &["um", "uh", "erm"];

/// Conversational backchannels (ADR 0014): preserve when they are the whole
/// utterance; strip only when they trail substantive content in the same span.
const BACKCHANNELS_RU: &[&str] = &["угу", "ага", "мгм"];
const BACKCHANNELS_EN: &[&str] = &["uhhuh", "mhm", "mmhmm"];

/// Short invitation / encouragement echoes. Adjacent repeats collapse to one
/// (`Ну давай. Давай, давай.` → `Ну давай.`). Not general word-dedup.
const ECHO_RU: &[&str] = &[
    "давай", "ладно", "хорошо", "погоди", "смотри", "слушай", "кидай", "валяй",
];
const ECHO_EN: &[&str] = &["okay", "ok", "right", "sure", "go", "come"];

/// Meaningful discourse markers that must never be touched by filler / false
/// start rules, even when they superficially match (ADR 0012 §1 "Never remove").
const PROTECTED_RU: &[(&str, &str)] = &[("ну", "да"), ("ну", "конечно"), ("вот", "именно")];
const PROTECTED_EN: &[(&str, &str)] = &[("of", "course"), ("exactly", "right"), ("well", "yeah")];

/// ru-priority default, mirrors `vd-fix-asr`'s `Language::En => En, _ => Ru`.
fn resolved_lang(language: Language) -> Language {
    match language {
        Language::En => Language::En,
        _ => Language::Ru,
    }
}

fn fillers_for(language: Language) -> &'static [&'static str] {
    match resolved_lang(language) {
        Language::En => FILLERS_EN,
        _ => FILLERS_RU,
    }
}

fn is_filler_for(word: &str, language: Language) -> bool {
    let lower = word.to_lowercase();
    fillers_for(language).contains(&lower.as_str())
}

fn backchannels_for(language: Language) -> &'static [&'static str] {
    match resolved_lang(language) {
        Language::En => BACKCHANNELS_EN,
        _ => BACKCHANNELS_RU,
    }
}

fn is_backchannel_for(word: &str, language: Language) -> bool {
    let lower = word.to_lowercase();
    backchannels_for(language).contains(&lower.as_str())
}

fn echoes_for(language: Language) -> &'static [&'static str] {
    match resolved_lang(language) {
        Language::En => ECHO_EN,
        _ => ECHO_RU,
    }
}

fn is_echo_for(word: &str, language: Language) -> bool {
    let lower = word.to_lowercase();
    echoes_for(language).contains(&lower.as_str())
}

/// Protection is checked against **both** language tables regardless of the
/// active `--language`, since under-protecting a discourse marker is worse
/// than a filler survivor list that never fires. See ADR 0012 §1.
fn is_protected_pair(a: &str, b: &str) -> bool {
    let la = a.to_lowercase();
    let lb = b.to_lowercase();
    PROTECTED_RU
        .iter()
        .chain(PROTECTED_EN.iter())
        .any(|(x, y)| *x == la && *y == lb)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Chunk {
    Word(String),
    Sep(String),
}

impl Chunk {
    fn as_str(&self) -> &str {
        match self {
            Self::Word(s) | Self::Sep(s) => s,
        }
    }
}

/// Maximal-munch word / separator tokenizer. A "word" is a run of
/// alphanumeric characters (unicode-aware, so Cyrillic works); everything
/// else (whitespace, punctuation, ellipses) is a separator run. Concatenating
/// the returned chunks in order always reconstructs the original text.
fn tokenize(text: &str) -> Vec<Chunk> {
    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut chunks = Vec::new();
    let mut pos = 0usize;
    while pos < chars.len() {
        let want_word = is_word_char(chars[pos].1);
        let start_byte = chars[pos].0;
        let mut end = pos + 1;
        while end < chars.len() && is_word_char(chars[end].1) == want_word {
            end += 1;
        }
        let end_byte = chars.get(end).map_or(text.len(), |(b, _)| *b);
        let slice = &text[start_byte..end_byte];
        chunks.push(if want_word {
            Chunk::Word(slice.to_string())
        } else {
            Chunk::Sep(slice.to_string())
        });
        pos = end;
    }
    chunks
}

fn render(chunks: &[Chunk]) -> String {
    chunks.iter().map(Chunk::as_str).collect()
}

/// A separator that (once whitespace is stripped) is nothing but 2+ dots or
/// a unicode ellipsis — a spoken pause, not sentence punctuation.
fn is_dot_run(raw: &str) -> bool {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.replace('…', "...");
    !compact.is_empty() && compact.chars().all(|c| c == '.') && compact.len() >= 2
}

/// Collapse a separator's punctuation core while preserving whether it had
/// leading / trailing whitespace, so words stay properly spaced after a
/// neighboring chunk is removed. Handles the artifacts left behind by
/// dropping a filler or a false-start word (double commas, doubled ellipses).
fn normalize_sep(raw: &str) -> String {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.replace('…', "...");
    let has_leading_space = raw.chars().next().is_some_and(char::is_whitespace);
    let has_trailing_space = raw.chars().last().is_some_and(char::is_whitespace);

    let core = if compact.is_empty() {
        String::new()
    } else if compact.contains(',') {
        // Comma wins when a merge mixes a comma with leftover dots (e.g. a
        // filler removed between "word," and "... word" collapses to a
        // single mild pause, not "word,... word").
        ",".to_string()
    } else if compact.chars().all(|c| c == '.') && compact.len() >= 2 {
        "...".to_string()
    } else {
        compact
    };

    if core.is_empty() {
        if raw.chars().any(char::is_whitespace) {
            " ".to_string()
        } else {
            String::new()
        }
    } else {
        let mut out = String::new();
        if has_leading_space {
            out.push(' ');
        }
        out.push_str(&core);
        if has_trailing_space {
            out.push(' ');
        }
        out
    }
}

fn merge_seps(chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::with_capacity(chunks.len());
    for c in chunks {
        match (&c, out.last_mut()) {
            (Chunk::Sep(s), Some(Chunk::Sep(prev))) => prev.push_str(s),
            _ => out.push(c),
        }
    }
    out
}

fn normalize_all_seps(chunks: Vec<Chunk>) -> Vec<Chunk> {
    chunks
        .into_iter()
        .map(|c| match c {
            Chunk::Sep(s) => Chunk::Sep(normalize_sep(&s)),
            w @ Chunk::Word(_) => w,
        })
        .collect()
}

/// Empty hesitations: `word1 … filler … word2` → `word1, word2` (ADR 0012 §1
/// example: `Ну... эээ... да...` → `Ну, да...`). Deliberately narrow: matches
/// exactly one filler flanked by pause separators between two real words.
/// Applied ahead of the general filler pass so the leftover pause becomes a
/// comma instead of a doubled ellipsis.
fn apply_empty_hesitation(chunks: &[Chunk], language: Language) -> Vec<Chunk> {
    let mut out = Vec::with_capacity(chunks.len());
    let mut i = 0;
    while i < chunks.len() {
        if i + 4 < chunks.len() {
            if let (
                Chunk::Word(w1),
                Chunk::Sep(s1),
                Chunk::Word(w2),
                Chunk::Sep(s2),
                Chunk::Word(_),
            ) = (
                &chunks[i],
                &chunks[i + 1],
                &chunks[i + 2],
                &chunks[i + 3],
                &chunks[i + 4],
            ) {
                if !is_filler_for(w1, language)
                    && is_dot_run(s1)
                    && is_filler_for(w2, language)
                    && is_dot_run(s2)
                {
                    out.push(chunks[i].clone());
                    out.push(Chunk::Sep(", ".to_string()));
                    i += 4;
                    continue;
                }
            }
        }
        out.push(chunks[i].clone());
        i += 1;
    }
    out
}

/// Remove isolated fillers (all modes except `off`); collapse a run of 2+
/// consecutive fillers to a single occurrence (`light`) or drop the whole
/// run (`normal` / `aggressive`).
fn collapse_fillers(chunks: &[Chunk], language: Language, mode: Mode) -> Vec<Chunk> {
    let mut out = Vec::with_capacity(chunks.len());
    let mut i = 0;
    while i < chunks.len() {
        if let Chunk::Word(w) = &chunks[i] {
            if is_filler_for(w, language) {
                let mut j = i;
                let mut count = 1u32;
                while j + 2 < chunks.len() {
                    if let (Chunk::Sep(_), Chunk::Word(w2)) = (&chunks[j + 1], &chunks[j + 2]) {
                        if is_filler_for(w2, language) {
                            j += 2;
                            count += 1;
                            continue;
                        }
                    }
                    break;
                }
                let keep_one = mode == Mode::Light && count >= 2;
                if keep_one {
                    out.push(chunks[i].clone());
                    i = j + 1;
                } else if out.is_empty() {
                    // Full removal at the very start of the text — also
                    // swallow the trailing separator so no orphaned leading
                    // punctuation remains ("эээ, тест" -> "тест", not ", тест").
                    i = match chunks.get(j + 1) {
                        Some(Chunk::Sep(_)) => j + 2,
                        _ => j + 1,
                    };
                } else {
                    i = j + 1;
                }
                continue;
            }
        }
        out.push(chunks[i].clone());
        i += 1;
    }
    out
}

fn is_repeat(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// If `w1` was capitalized and `w2` is lowercase, carry the capitalization
/// over to `w2` (`Я... я думаю` → `Я думаю`, not `я думаю`).
fn recapitalize(w1: &str, w2: &str) -> String {
    let w1_cap = w1.chars().next().is_some_and(char::is_uppercase);
    let w2_lower = w2.chars().next().is_some_and(char::is_lowercase);
    if w1_cap && w2_lower {
        let mut chars = w2.chars();
        chars.next().map_or_else(
            || w2.to_string(),
            |f| f.to_uppercase().collect::<String>() + chars.as_str(),
        )
    } else {
        w2.to_string()
    }
}

/// False starts: `word... word continuation` → `word continuation`, only
/// when the repeat is exact and separated by a spoken pause. `normal` /
/// `aggressive` only (ADR 0012 §1: "only when clearly accidental"). Guarded
/// by the protected-phrase list so e.g. `Ну... ну да` is left untouched.
fn collapse_false_starts(chunks: &[Chunk]) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::with_capacity(chunks.len());
    let mut i = 0;
    while i < chunks.len() {
        let mut handled = false;
        if i + 2 < chunks.len() {
            if let (Chunk::Word(w1), Chunk::Sep(s), Chunk::Word(w2)) =
                (&chunks[i], &chunks[i + 1], &chunks[i + 2])
            {
                if is_dot_run(s) && is_repeat(w1, w2) {
                    let w3 = chunks.get(i + 4).and_then(|c| match c {
                        Chunk::Word(w) => Some(w.as_str()),
                        Chunk::Sep(_) => None,
                    });
                    let guarded = w3.is_some_and(|w3| is_protected_pair(w2, w3));
                    if !guarded {
                        out.push(Chunk::Word(recapitalize(w1, w2)));
                        i += 3;
                        handled = true;
                    }
                }
            }
        }
        if !handled {
            out.push(chunks[i].clone());
            i += 1;
        }
    }
    out
}

/// Drop trailing `угу` / `ага` / … after substantive content; keep when the
/// whole span is only backchannels (meaningful sole-turn ack). Light+.
fn strip_trailing_backchannels(chunks: &[Chunk], language: Language) -> Vec<Chunk> {
    let word_idxs: Vec<usize> = chunks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            Chunk::Word(_) => Some(i),
            Chunk::Sep(_) => None,
        })
        .collect();
    if word_idxs.is_empty() {
        return chunks.to_vec();
    }

    let has_substance = word_idxs.iter().any(|&i| match &chunks[i] {
        Chunk::Word(w) => !is_backchannel_for(w, language),
        Chunk::Sep(_) => false,
    });
    if !has_substance {
        return chunks.to_vec();
    }

    let mut first_trailing = word_idxs.len();
    while first_trailing > 0 {
        let idx = word_idxs[first_trailing - 1];
        match &chunks[idx] {
            Chunk::Word(w) if is_backchannel_for(w, language) => first_trailing -= 1,
            _ => break,
        }
    }
    if first_trailing == word_idxs.len() {
        return chunks.to_vec();
    }

    let cut_at = word_idxs[first_trailing];
    let mut out = chunks[..cut_at].to_vec();
    if let Some(Chunk::Sep(s)) = out.last_mut() {
        let trimmed = trim_sep_after_backchannel_strip(s);
        if trimmed.is_empty() {
            out.pop();
        } else {
            *s = trimmed;
        }
    }
    out
}

/// After dropping trailing backchannels, keep sentence-final `.!?` on the
/// last substantive word; drop a dangling comma/space bridge (`word, угу`).
fn trim_sep_after_backchannel_strip(raw: &str) -> String {
    let has_leading_space = raw.chars().next().is_some_and(char::is_whitespace);
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() || compact == "," {
        return String::new();
    }
    let core: String = compact
        .chars()
        .take_while(|c| matches!(c, '.' | '!' | '?' | '…'))
        .collect();
    if core.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    if has_leading_space {
        out.push(' ');
    }
    out.push_str(&core);
    out
}

fn is_light_punct_sep(raw: &str) -> bool {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    compact.is_empty()
        || compact
            .chars()
            .all(|c| matches!(c, ',' | '.' | '!' | '?' | '…'))
}

fn sep_has_sentence_end(raw: &str) -> bool {
    raw.chars().any(|c| matches!(c, '.' | '!' | '?' | '…'))
}

/// Collapse adjacent invitation/encouragement echoes (allowlisted):
/// `Давай, давай` → `Давай`; `Ну давай. Давай, давай.` → `Ну давай.`
fn collapse_echo_repeats(chunks: &[Chunk], language: Language) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::with_capacity(chunks.len());
    let mut i = 0;
    while i < chunks.len() {
        if let Chunk::Word(w) = &chunks[i] {
            if is_echo_for(w, language) {
                let first = w.clone();
                out.push(Chunk::Word(first.clone()));
                i += 1;
                let mut swallowed = false;
                let mut need_sentence_end = false;
                while i + 1 < chunks.len() {
                    if let (Chunk::Sep(s), Chunk::Word(w2)) = (&chunks[i], &chunks[i + 1]) {
                        if is_light_punct_sep(s) && is_repeat(&first, w2) {
                            if sep_has_sentence_end(s) {
                                need_sentence_end = true;
                            }
                            swallowed = true;
                            i += 2;
                            continue;
                        }
                    }
                    break;
                }
                if swallowed && need_sentence_end {
                    let next_has = matches!(
                        chunks.get(i),
                        Some(Chunk::Sep(s)) if sep_has_sentence_end(s)
                    );
                    if !next_has {
                        out.push(Chunk::Sep(". ".to_string()));
                    }
                }
                continue;
            }
        }
        out.push(chunks[i].clone());
        i += 1;
    }
    out
}

/// Run the full ADR 0012 §1 rule pipeline over one span of text.
pub fn fix_text(text: &str, language: Language, mode: Mode) -> FixResult {
    if mode == Mode::Off {
        return FixResult {
            text: text.to_string(),
            changed: false,
        };
    }

    let chunks = tokenize(text);
    let chunks = apply_empty_hesitation(&chunks, language);
    let chunks = collapse_fillers(&chunks, language, mode);
    let chunks = strip_trailing_backchannels(&chunks, language);
    let chunks = collapse_echo_repeats(&chunks, language);
    let chunks = normalize_all_seps(merge_seps(chunks));

    let chunks = if mode >= Mode::Normal {
        normalize_all_seps(merge_seps(collapse_false_starts(&chunks)))
    } else {
        chunks
    };

    let mid = render(&chunks);
    let out = apply_glued_onset_pass(&mid);
    let changed = out != text;
    FixResult { text: out, changed }
}

/// ADR 0014 §3: collapse glued onset tokens (`Ччисто` → `Чисто`) via `vd-text`.
/// Scoped pass — does not re-run filler/orphan/false-start (those stay in this module).
fn apply_glued_onset_pass(text: &str) -> String {
    tokenize(text)
        .into_iter()
        .map(|c| match c {
            Chunk::Word(w) => {
                vd_text::disfluency::patterns::collapse_glued_onset(&w).unwrap_or(w)
            }
            Chunk::Sep(s) => s,
        })
        .collect()
}
