//! Pattern detection for disfluency artifacts (stuttering, orphan letters, repeated words).

use crate::rule_engine::Confidence;

/// Detect repeated syllables (stuttering): я-я, по-по, н-н-ну.
pub fn detect_stutter(text: &str) -> Option<(usize, usize, Confidence)> {
    // Simple pattern: X-X or X-X-X format (repeated syllables with hyphen)
    let parts: Vec<&str> = text.split('-').collect();

    if parts.len() < 2 {
        return None;
    }

    // Check if first parts are identical (stuttering pattern)
    if parts.len() >= 2 && parts[0] == parts[1] && !parts[0].is_empty() {
        let confidence = if parts.len() >= 3 && parts[0] == parts[2] {
            Confidence::Certain // Triple repeat: very sure
        } else {
            Confidence::Certain // Double repeat: also confident
        };
        return Some((0, text.len(), confidence));
    }

    None
}

/// Detect orphan letters — single letters used as hesitation.
/// Confidence depends on context and length.
pub fn detect_orphan_letter(text: &str) -> Option<(usize, usize, Confidence)> {
    let trimmed = text.trim();

    // Get only alphabetic part (skip trailing punctuation)
    let alpha_part: String = trimmed
        .chars()
        .take_while(|c| c.is_alphabetic())
        .collect();

    // Must be a single alphabetic character
    if alpha_part.len() == 0 || alpha_part.chars().count() != 1 {
        return None;
    }

    Some((0, text.len(), Confidence::Likely))
}

/// Detect repeated words (филлеры ну... ну..., да-да-да).
pub fn detect_repeated_word(text: &str) -> Option<(usize, usize, Confidence)> {
    let parts: Vec<&str> = text.split(|c: char| c == '-' || c.is_whitespace()).collect();
    let filtered: Vec<&str> = parts.into_iter().filter(|s| !s.is_empty()).collect();

    if filtered.len() < 2 {
        return None;
    }

    // Check if first words repeat
    if filtered[0] == filtered[1] {
        // For common fillers (ну, да, вот), high confidence
        let confidence = match filtered[0].to_lowercase().as_str() {
            "ну" | "да" | "вот" | "так" => Confidence::Likely,
            _ => Confidence::Certain,
        };
        return Some((0, text.len(), confidence));
    }

    None
}

/// Detect false starts: word... word-continuation (я... я думаю).
pub fn detect_false_start(word1: &str, word2: &str) -> Option<Confidence> {
    let w1_normalized = word1.trim().to_lowercase();
    let w2_normalized = word2.trim().to_lowercase();

    // Check if word2 starts with word1
    if w2_normalized.starts_with(&w1_normalized) && w1_normalized.len() < w2_normalized.len() {
        // False start: prefix matches and continuation exists
        return Some(Confidence::Certain);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stutter_detection() {
        // Double stutter
        let (_, _, conf) = detect_stutter("я-я").expect("should detect я-я");
        assert_eq!(conf, Confidence::Certain);

        // Triple stutter
        let (_, _, conf) = detect_stutter("н-н-ну").expect("should detect н-н-ну");
        assert_eq!(conf, Confidence::Certain);

        // Not a stutter
        assert!(detect_stutter("по-тому").is_none());
    }

    #[test]
    fn test_orphan_letter_detection() {
        // Single letter
        let (_, _, conf) = detect_orphan_letter("я").expect("should detect single я");
        assert_eq!(conf, Confidence::Likely);

        // Letter with ellipsis
        let (_, _, conf) = detect_orphan_letter("э...").expect("should detect э...");
        assert_eq!(conf, Confidence::Likely);

        // Not an orphan letter
        assert!(detect_orphan_letter("привет").is_none());
    }

    #[test]
    fn test_repeated_word_detection() {
        // Repeated with hyphen
        let (_, _, conf) = detect_repeated_word("да-да").expect("should detect да-да");
        assert_eq!(conf, Confidence::Likely);

        // Repeated with space
        let (_, _, conf) = detect_repeated_word("ну ну").expect("should detect ну ну");
        assert_eq!(conf, Confidence::Likely);

        // Not repeated
        assert!(detect_repeated_word("ну сказать").is_none());
    }

    #[test]
    fn test_false_start_detection() {
        // я -> я думаю
        let conf = detect_false_start("я", "я думаю").expect("should detect false start");
        assert_eq!(conf, Confidence::Certain);

        // дума -> думаю
        let conf = detect_false_start("дума", "думаю").expect("should detect false start");
        assert_eq!(conf, Confidence::Certain);

        // No false start
        assert!(detect_false_start("я", "думаю").is_none());
    }
}
