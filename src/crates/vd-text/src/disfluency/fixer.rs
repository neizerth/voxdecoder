//! Disfluency fixer — apply cleanup with confidence-based mode gating.

use crate::rule_engine::Confidence;
use super::detector::{DisfluencyDetector, ArtifactType, DisfluencyHit};

/// Cleanup mode for disfluency (ADR 012 + ADR 014).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Mode {
    Off,
    Light,
    Normal,
    Aggressive,
}

impl Mode {
    /// Check if this mode allows a rule at given confidence.
    pub fn allows(&self, confidence: Confidence) -> bool {
        match (self, confidence) {
            (Mode::Off, _) => false,
            (Mode::Light, Confidence::Certain) => true,
            (Mode::Light, _) => false,
            (Mode::Normal, Confidence::Certain | Confidence::Likely) => true,
            (Mode::Normal, Confidence::Unsafe) => false,
            (Mode::Aggressive, _) => true,
        }
    }
}

/// Disfluency fixer with mode gating.
pub struct DisfluencyFixer {
    detector: DisfluencyDetector,
    mode: Mode,
    protected_phrases: Vec<String>,
}

impl DisfluencyFixer {
    pub fn new(language: &str, mode: Mode) -> Self {
        Self {
            detector: DisfluencyDetector::new(language),
            mode,
            protected_phrases: Self::default_protected_phrases(language),
        }
    }

    /// Protected phrases that should never be removed (meaningful discourse markers).
    fn default_protected_phrases(language: &str) -> Vec<String> {
        match language {
            "ru" => vec![
                "ну да".to_string(),
                "ну конечно".to_string(),
                "вот именно".to_string(),
                "да да".to_string(),
                "угу".to_string(),
                "ага".to_string(),
            ],
            "en" => vec![
                "you know".to_string(),
                "i mean".to_string(),
                "sort of".to_string(),
                "kind of".to_string(),
                "yeah".to_string(),
                "yeah yeah".to_string(),
            ],
            _ => vec![],
        }
    }

    /// Check if phrase is protected (should never be removed).
    fn is_protected(&self, text: &str) -> bool {
        let lower = text.trim().to_lowercase();
        self.protected_phrases.iter().any(|p| p == &lower)
    }

    /// Detect disfluency artifacts in text (word-level analysis).
    pub fn detect(&self, words: &[&str]) -> Vec<DisfluencyHit> {
        let mut hits = Vec::new();

        // Single-word detection
        for (i, word) in words.iter().enumerate() {
            let trimmed = word.trim();

            // Check filler
            if let Some(conf) = self.detector.is_filler(trimmed) {
                if !self.is_protected(trimmed) && self.mode.allows(conf) {
                    hits.push(DisfluencyHit {
                        artifact_type: ArtifactType::Filler,
                        text: word.to_string(),
                        start: 0,
                        end: word.len(),
                        confidence: conf,
                        rule_id: "filler".to_string(),
                    });
                }
            }

            // Check orphan letter
            if let Some(conf) = self.detector.is_orphan_letter(trimmed) {
                if self.mode.allows(conf) {
                    hits.push(DisfluencyHit {
                        artifact_type: ArtifactType::OrphanLetter,
                        text: word.to_string(),
                        start: 0,
                        end: word.len(),
                        confidence: conf,
                        rule_id: "orphan_letter".to_string(),
                    });
                }
            }

            // Check stutter
            if let Some(conf) = self.detector.is_stutter(trimmed) {
                if self.mode.allows(conf) {
                    hits.push(DisfluencyHit {
                        artifact_type: ArtifactType::Stutter,
                        text: word.to_string(),
                        start: 0,
                        end: word.len(),
                        confidence: conf,
                        rule_id: "stutter".to_string(),
                    });
                }
            }

            // Check repeated word
            if let Some(conf) = self.detector.is_repeated_word(trimmed) {
                if !self.is_protected(trimmed) && self.mode.allows(conf) {
                    hits.push(DisfluencyHit {
                        artifact_type: ArtifactType::RepeatedWord,
                        text: word.to_string(),
                        start: 0,
                        end: word.len(),
                        confidence: conf,
                        rule_id: "repeated_word".to_string(),
                    });
                }
            }

            // Check false start (with next word)
            if i + 1 < words.len() {
                let next = words[i + 1].trim();
                if let Some(conf) = self.detector.is_false_start(trimmed, next) {
                    if self.mode.allows(conf) && self.mode != Mode::Light {
                        hits.push(DisfluencyHit {
                            artifact_type: ArtifactType::FalseStart,
                            text: word.to_string(),
                            start: 0,
                            end: word.len(),
                            confidence: conf,
                            rule_id: "false_start".to_string(),
                        });
                    }
                }
            }
        }

        // Multi-word detection (empty hesitation)
        for window in words.windows(3) {
            if let Some(conf) = self.detector.is_empty_hesitation(window) {
                if self.mode.allows(conf) {
                    hits.push(DisfluencyHit {
                        artifact_type: ArtifactType::EmptyHesitation,
                        text: format!("{} {} {}", window[0], window[1], window[2]),
                        start: 0,
                        end: 0,
                        confidence: conf,
                        rule_id: "empty_hesitation".to_string(),
                    });
                }
            }
        }

        hits
    }

    /// Apply cleanup: return text with artifacts removed/collapsed.
    pub fn fix_text(&self, text: &str) -> (String, Vec<DisfluencyHit>) {
        let words: Vec<&str> = text.split_whitespace().collect();
        let hits = self.detect(&words);

        // For now, return original text and hits
        // In full implementation, this would apply transformations based on hits
        (text.to_string(), hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_allows() {
        assert!(!Mode::Off.allows(Confidence::Certain));
        assert!(Mode::Light.allows(Confidence::Certain));
        assert!(!Mode::Light.allows(Confidence::Likely));
        assert!(Mode::Normal.allows(Confidence::Likely));
        assert!(Mode::Aggressive.allows(Confidence::Unsafe));
    }

    #[test]
    fn test_filler_detection_by_mode() {
        let fixer_light = DisfluencyFixer::new("ru", Mode::Light);
        let fixer_normal = DisfluencyFixer::new("ru", Mode::Normal);

        let words = vec!["я", "ну", "думаю"];

        // "ну" is Likely, so Light mode should not flag it
        let hits_light = fixer_light.detect(&words);
        assert!(!hits_light.iter().any(|h| h.rule_id == "filler"));

        // Normal mode should flag it
        let hits_normal = fixer_normal.detect(&words);
        assert!(hits_normal.iter().any(|h| h.rule_id == "filler"));
    }

    #[test]
    fn test_orphan_letter_detection() {
        // Orphan letters are Likely confidence, so need Normal mode to detect
        let fixer = DisfluencyFixer::new("ru", Mode::Normal);
        let words = vec!["я", "думаю"];

        let hits = fixer.detect(&words);
        // Should detect orphan letter "я" in Normal mode
        assert!(!hits.is_empty());
        assert!(hits.iter().any(|h| h.artifact_type == ArtifactType::OrphanLetter));
    }

    #[test]
    fn test_filler_by_mode() {
        let fixer_light = DisfluencyFixer::new("ru", Mode::Light);
        let fixer_normal = DisfluencyFixer::new("ru", Mode::Normal);
        let words = vec!["эээ"];

        // Light should remove certain fillers
        let hits_light = fixer_light.detect(&words);
        assert!(hits_light.iter().any(|h| h.rule_id == "filler"));

        // Normal should also remove certain fillers
        let hits_normal = fixer_normal.detect(&words);
        assert!(hits_normal.iter().any(|h| h.rule_id == "filler"));
    }
}
