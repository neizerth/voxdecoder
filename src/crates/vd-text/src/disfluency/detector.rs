//! Disfluency detector — combine lexical, structural, and morphological signals.

use crate::rule_engine::Confidence;
use super::dictionary::DisfluencyDictionary;
use super::patterns;

/// Result of disfluency detection.
#[derive(Debug, Clone)]
pub struct DisfluencyHit {
    pub artifact_type: ArtifactType,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub confidence: Confidence,
    pub rule_id: String,
}

/// Type of disfluency artifact detected.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArtifactType {
    Filler,
    OrphanLetter,
    Stutter,
    GluedOnset,
    RepeatedWord,
    FalseStart,
    EmptyHesitation,
}

/// Detector combining multiple signals for disfluency.
pub struct DisfluencyDetector {
    dict: DisfluencyDictionary,
}

impl DisfluencyDetector {
    pub fn new(language: &str) -> Self {
        Self {
            dict: DisfluencyDictionary::for_language(language),
        }
    }

    /// Detect if word is a filler (lexical signal).
    pub fn is_filler(&self, word: &str) -> Option<Confidence> {
        self.dict.fillers.confidence(word)
    }

    /// Detect if text is an orphan letter (lexical + structural).
    pub fn is_orphan_letter(&self, text: &str) -> Option<Confidence> {
        // Get only alphabetic part (skip trailing punctuation)
        let trimmed = text.trim();
        let alpha_part: String = trimmed
            .chars()
            .take_while(|c| c.is_alphabetic())
            .collect();

        // Must be single character and in dictionary
        if !alpha_part.is_empty() && self.dict.orphan_letters.is_orphan(&alpha_part) {
            patterns::detect_orphan_letter(text).map(|(_, _, conf)| conf)
        } else {
            None
        }
    }

    /// Detect if text is stuttering (structural pattern).
    pub fn is_stutter(&self, text: &str) -> Option<Confidence> {
        patterns::detect_stutter(text).map(|(_, _, conf)| conf)
    }

    /// Detect glued onset (`Ччисто`) — ADR 0014 §3.
    pub fn is_glued_onset(&self, text: &str) -> Option<Confidence> {
        patterns::detect_glued_onset(text).map(|(_, _, conf)| conf)
    }

    /// Detect if text is repeated words (lexical + structural).
    pub fn is_repeated_word(&self, text: &str) -> Option<Confidence> {
        patterns::detect_repeated_word(text).map(|(_, _, conf)| conf)
    }

    /// Detect if two tokens form a false start (structural + context).
    pub fn is_false_start(&self, word1: &str, word2: &str) -> Option<Confidence> {
        patterns::detect_false_start(word1, word2)
    }

    /// Detect empty hesitation chain: word... filler... word (structural).
    pub fn is_empty_hesitation(&self, tokens: &[&str]) -> Option<Confidence> {
        if tokens.len() < 3 {
            return None;
        }

        // Pattern: word ... filler ... word
        let first_is_word = !tokens[0].trim().is_empty() && !self.is_filler(tokens[0]).is_some();
        let middle_is_filler = self.is_filler(tokens[1]).is_some();
        let last_is_word = !tokens[2].trim().is_empty() && !self.is_filler(tokens[2]).is_some();

        if first_is_word && middle_is_filler && last_is_word {
            Some(Confidence::Certain)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filler_detection_ru() {
        let detector = DisfluencyDetector::new("ru");
        assert_eq!(detector.is_filler("эээ"), Some(Confidence::Certain));
        assert_eq!(detector.is_filler("ну"), Some(Confidence::Likely));
        assert_eq!(detector.is_filler("привет"), None);
    }

    #[test]
    fn test_orphan_letter_detection() {
        let detector = DisfluencyDetector::new("ru");
        assert!(detector.is_orphan_letter("я").is_some());
        assert!(detector.is_orphan_letter("э...").is_some());
        assert!(detector.is_orphan_letter("привет").is_none());
    }

    #[test]
    fn test_stutter_detection() {
        let detector = DisfluencyDetector::new("ru");
        assert!(detector.is_stutter("я-я").is_some());
        assert!(detector.is_stutter("по-по").is_some());
        assert!(detector.is_stutter("по-тому").is_none());
    }

    #[test]
    fn test_repeated_word_detection() {
        let detector = DisfluencyDetector::new("ru");
        assert!(detector.is_repeated_word("да-да").is_some());
        assert!(detector.is_repeated_word("ну ну").is_some());
        assert!(detector.is_repeated_word("ну сказать").is_none());
    }

    #[test]
    fn test_false_start_detection() {
        let detector = DisfluencyDetector::new("ru");
        assert!(detector.is_false_start("я", "я думаю").is_some());
        assert!(detector.is_false_start("дума", "думаю").is_some());
        assert!(detector.is_false_start("я", "думаю").is_none());
    }

    #[test]
    fn test_empty_hesitation() {
        let detector = DisfluencyDetector::new("ru");
        let tokens = vec!["я", "эээ", "думаю"];
        assert!(detector.is_empty_hesitation(&tokens).is_some());

        let tokens_no_hesitation = vec!["я", "и", "думаю"];
        assert!(detector.is_empty_hesitation(&tokens_no_hesitation).is_none());
    }
}
