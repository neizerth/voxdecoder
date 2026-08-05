//! Shared mix / merged recording detection.
//!
//! **STUB — implementation tracked separately (ADR 0017 P1-B).** Signature + full token table
//! spec below; fill in the body against the doc-comment and skill.md, do not invent new tokens.

/// Is `name` (already run through [`crate::strip_basename_noise`], case-insensitive) a
/// shared-mix / merged-recording token?
///
/// Source: `skills/vd-meeting/skill.md` **Filename heuristics → Shared mix (`role: room` /
/// `merged`)**. Treat as the common room recording when the name **contains** (not necessarily
/// equals) one of these tokens, case-insensitively:
///
/// ```text
/// mix, mixed, merged, all, room, full, combined, common, overall, together,
/// весь, общ, микс, слит, полный
/// ```
///
/// Examples from skill.md: `meeting_mix.wav`, `all.mp4`, `merged_track.m4a`,
/// `общая_запись.wav` (contains `общ`) should all match; a plain person name
/// (`Игорь`, `alice`) must not.
///
/// Implementation note: match on substring containment of the token within the (lowercased)
/// name, not exact word equality — `общая_запись` matching via the `общ` prefix depends on
/// this. Watch for false positives on real names that happen to contain a short token
/// (e.g. `общ` inside an unrelated word) — skill.md accepts this trade-off for `vd-meeting`'s
/// AI-agent flow (a human confirms afterward); the same trade-off applies here since
/// `classify_inputs` output is also confirmed via the `--interactive` wizard, never auto-applied.
pub fn is_mix_token(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    const TOKENS: &[&str] = &[
        // English
        "mix",
        "mixed",
        "merged",
        "all",
        "room",
        "full",
        "combined",
        "common",
        "overall",
        "together",
        // Russian
        "весь",
        "общ",
        "микс",
        "слит",
        "полный",
    ];
    TOKENS.iter().any(|token| lower.contains(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_documented_mix_tokens() {
        for name in [
            "meeting mix",
            "all",
            "merged track",
            "общая запись",
        ] {
            assert!(is_mix_token(name), "expected {name:?} to be a mix token");
        }
    }

    #[test]
    fn plain_names_are_not_mix_tokens() {
        for name in ["Игорь", "alice", "Владимир"] {
            assert!(!is_mix_token(name), "did not expect {name:?} to be a mix token");
        }
    }
}
