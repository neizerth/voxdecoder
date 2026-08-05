//! Strip timestamp-looking tokens and separator noise from a filename stem.
//!
//! Reference implementation — the pattern the rest of this crate's stubs follow.

/// Strip timestamp-looking tokens and normalize separator whitespace in a filename **stem**
/// (extension already removed by the caller, e.g. via `Path::file_stem()`), leaving a
/// candidate display name.
///
/// Rule (`skills/vd-meeting/skill.md` **Filename heuristics**): "strip timestamp-looking
/// tokens and extra whitespace from the basename to get a name."
///
/// Approach: split on common separators (space, `_`, `-`, `.`), drop tokens that are purely
/// digits of a length matching common date/time chunks (`YYYY`=4, `YYYYMMDD`=8, `HHMMSS`=6,
/// `MM`/`DD`/`HH`=2), rejoin the rest with single spaces.
///
/// Known gap (left for a follow-up, not blocking): month-name dates (`Jan`, `15`) and other
/// non-numeric timestamp shapes are not stripped — only digit-run tokens are recognized.
pub fn strip_basename_noise(stem: &str) -> String {
    let kept: Vec<&str> = stem
        .split(|c: char| c == '_' || c == '-' || c == '.' || c.is_whitespace())
        .filter(|tok| !tok.is_empty() && !is_timestamp_chunk(tok))
        .collect();
    kept.join(" ")
}

fn is_timestamp_chunk(tok: &str) -> bool {
    let all_digits = !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit());
    all_digits && matches!(tok.len(), 2 | 4 | 6 | 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_date_and_time_tokens() {
        assert_eq!(strip_basename_noise("Игорь_2024-01-15_10-30"), "Игорь");
    }

    #[test]
    fn strips_compact_yyyymmdd() {
        assert_eq!(strip_basename_noise("meeting_mix_20240115"), "meeting mix");
    }

    #[test]
    fn leaves_plain_names_untouched() {
        assert_eq!(strip_basename_noise("Владимир"), "Владимир");
    }

    #[test]
    fn collapses_multiple_separators() {
        assert_eq!(strip_basename_noise("alice__2024"), "alice");
    }
}
