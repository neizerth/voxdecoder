//! Private layout backend (paragraph grouping).

mod en;
mod ru;

use crate::models::Lexicon;
use crate::types::{Language, ParagraphDensity, TimeMap};

use super::signals::{discourse_break, soft_break, split_sentences};
use super::timemap::prefers_relaxed_breaks;

pub fn rewrite(
    text: &str,
    language: Language,
    density: ParagraphDensity,
    lexicon: &Lexicon,
    timemap: Option<&TimeMap>,
) -> String {
    let density = adjust_density(density, timemap);
    match language {
        Language::En | Language::De => en::layout(text, density, lexicon),
        Language::Ru | Language::Auto => ru::layout(text, density, lexicon),
    }
}

fn adjust_density(density: ParagraphDensity, timemap: Option<&TimeMap>) -> ParagraphDensity {
    match timemap {
        Some(map) if prefers_relaxed_breaks(map) => match density {
            ParagraphDensity::Compact => ParagraphDensity::Normal,
            ParagraphDensity::Normal | ParagraphDensity::Relaxed => ParagraphDensity::Relaxed,
        },
        _ => density,
    }
}

pub(super) fn layout_paragraphs(
    text: &str,
    density: ParagraphDensity,
    lexicon: &Lexicon,
) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Preserve existing paragraph blocks; layout inside each.
    let blocks: Vec<&str> = if trimmed.contains("\n\n") {
        trimmed
            .split("\n\n")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![trimmed]
    };

    let mut out_blocks = Vec::new();
    for block in blocks {
        let flat = block
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let sentences = split_sentences(&flat);
        if sentences.is_empty() {
            continue;
        }
        out_blocks.extend(group_sentences(&sentences, density, lexicon));
    }

    // Normalize accidental multi-blank runs by joining with exactly `\n\n`.
    out_blocks.join("\n\n")
}

fn group_sentences(
    sentences: &[String],
    density: ParagraphDensity,
    lexicon: &Lexicon,
) -> Vec<String> {
    let target = density.target_sentences();
    let max = density.max_sentences();
    let mut paras = Vec::new();
    let mut cur: Vec<&str> = Vec::new();

    for (idx, sent) in sentences.iter().enumerate() {
        let force_before = idx > 0
            && (discourse_break(sent, lexicon)
                || (soft_break(sent, lexicon) && cur.len() >= target.saturating_sub(1)));

        if force_before && !cur.is_empty() {
            paras.push(cur.join(" "));
            cur.clear();
        }

        cur.push(sent.as_str());

        let at_soft_target = cur.len() >= target
            && idx + 1 < sentences.len()
            && !discourse_break(&sentences[idx + 1], lexicon);
        let at_hard_max = cur.len() >= max;

        if (at_soft_target || at_hard_max) && idx + 1 < sentences.len() {
            // Prefer breaking before a discourse cue on the next sentence.
            if discourse_break(&sentences[idx + 1], lexicon) || at_hard_max || at_soft_target {
                paras.push(cur.join(" "));
                cur.clear();
            }
        }
    }

    if !cur.is_empty() {
        paras.push(cur.join(" "));
    }
    paras
}
