//! Disfluency dictionaries — fillers, orphan letters, false starts (language-specific).
//!
//! Extends LanguagePack with disfluency-specific patterns and confidence levels.

use crate::rule_engine::Confidence;
use std::collections::HashSet;

/// Filler word dictionary (language-specific).
#[derive(Debug, Clone)]
pub struct FillerDictionary {
    /// High-confidence fillers (remove immediately).
    pub certain: HashSet<String>,
    /// Medium-confidence fillers (remove in normal+ mode).
    pub likely: HashSet<String>,
}

impl FillerDictionary {
    /// Lookup filler confidence.
    pub fn confidence(&self, word: &str) -> Option<Confidence> {
        if self.certain.contains(word) {
            Some(Confidence::Certain)
        } else if self.likely.contains(word) {
            Some(Confidence::Likely)
        } else {
            None
        }
    }

    /// Check if word is a known filler.
    pub fn is_filler(&self, word: &str) -> bool {
        self.certain.contains(word) || self.likely.contains(word)
    }
}

/// Orphan letter dictionary (single letters used as hesitation).
#[derive(Debug, Clone)]
pub struct OrphanLetterDictionary {
    /// Letters that are almost always hesitations (я, э, м, н in Russian).
    pub certain: HashSet<String>,
    /// Letters that may be hesitations depending on context.
    pub likely: HashSet<String>,
}

impl OrphanLetterDictionary {
    /// Lookup orphan letter confidence.
    pub fn confidence(&self, letter: &str) -> Option<Confidence> {
        if self.certain.contains(letter) {
            Some(Confidence::Certain)
        } else if self.likely.contains(letter) {
            Some(Confidence::Likely)
        } else {
            None
        }
    }

    /// Check if single letter is an orphan letter.
    pub fn is_orphan(&self, letter: &str) -> bool {
        self.certain.contains(letter) || self.likely.contains(letter)
    }
}

/// Stutter pattern dictionary.
#[derive(Debug, Clone)]
pub struct StutterDictionary {
    /// Common repeated syllables (я-я, по-по, н-н).
    pub patterns: HashSet<String>,
}

impl StutterDictionary {
    /// Check if pattern is a known stutter.
    pub fn is_stutter(&self, pattern: &str) -> bool {
        self.patterns.contains(pattern)
    }
}

/// Language-specific disfluency dictionaries.
#[derive(Debug, Clone)]
pub struct DisfluencyDictionary {
    pub fillers: FillerDictionary,
    pub orphan_letters: OrphanLetterDictionary,
    pub stutters: StutterDictionary,
}

impl DisfluencyDictionary {
    /// Russian disfluency dictionary.
    pub fn russian() -> Self {
        Self {
            fillers: FillerDictionary {
                certain: vec!["эээ", "ммм", "эм", "мм", "ээ", "ага", "угу", "ахa"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                likely: vec!["ну", "вот", "типа", "как бы", "ну вот", "так сказать"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            orphan_letters: OrphanLetterDictionary {
                certain: vec!["я", "э", "м", "н"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                likely: vec!["а", "е", "у"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            stutters: StutterDictionary {
                patterns: vec!["я-я", "по-по", "н-н", "и-и", "ш-ш", "с-с", "т-т"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
        }
    }

    /// English disfluency dictionary.
    pub fn english() -> Self {
        Self {
            fillers: FillerDictionary {
                certain: vec!["uh", "um", "erm", "er", "uhh", "umm", "hmm"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                likely: vec!["like", "you know", "basically", "actually", "i mean", "sort of", "kind of"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            orphan_letters: OrphanLetterDictionary {
                certain: vec!["a", "e", "i", "o", "u"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                likely: vec!["the"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            stutters: StutterDictionary {
                patterns: vec!["t-t", "s-s", "d-d", "w-w", "b-b", "c-c", "p-p"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
        }
    }

    /// Get dictionary by language name.
    pub fn for_language(lang: &str) -> Self {
        match lang {
            "ru" | "russian" => Self::russian(),
            "en" | "english" => Self::english(),
            _ => Self::russian(), // Default to Russian
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filler_lookup() {
        let dict = DisfluencyDictionary::russian();
        assert_eq!(dict.fillers.confidence("эээ"), Some(Confidence::Certain));
        assert_eq!(dict.fillers.confidence("ну"), Some(Confidence::Likely));
        assert_eq!(dict.fillers.confidence("привет"), None);
    }

    #[test]
    fn test_orphan_letter_lookup() {
        let dict = DisfluencyDictionary::russian();
        assert_eq!(dict.orphan_letters.confidence("я"), Some(Confidence::Certain));
        assert_eq!(dict.orphan_letters.confidence("а"), Some(Confidence::Likely));
        assert_eq!(dict.orphan_letters.confidence("ы"), None);
    }

    #[test]
    fn test_stutter_detection() {
        let dict = DisfluencyDictionary::russian();
        assert!(dict.stutters.is_stutter("я-я"));
        assert!(dict.stutters.is_stutter("по-по"));
        assert!(!dict.stutters.is_stutter("по-тому"));
    }

    #[test]
    fn test_english_dictionary() {
        let dict = DisfluencyDictionary::english();
        assert_eq!(dict.fillers.confidence("um"), Some(Confidence::Certain));
        assert_eq!(dict.fillers.confidence("like"), Some(Confidence::Likely));
        assert!(dict.orphan_letters.is_orphan("a"));
    }
}
