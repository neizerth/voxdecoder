//! Disfluency fixer — apply cleanup with confidence-based mode gating.

use crate::rule_engine::Confidence;
use super::detector::{DisfluencyDetector, ArtifactType, DisfluencyHit};
use super::patterns;

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

        for (i, word) in words.iter().enumerate() {
            let trimmed = word.trim();

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

            if let Some(conf) = self.detector.is_glued_onset(trimmed) {
                if self.mode.allows(conf) {
                    hits.push(DisfluencyHit {
                        artifact_type: ArtifactType::GluedOnset,
                        text: word.to_string(),
                        start: 0,
                        end: word.len(),
                        confidence: conf,
                        rule_id: "glued_onset".to_string(),
                    });
                }
            }

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
        if self.mode == Mode::Off || text.is_empty() {
            return (text.to_string(), Vec::new());
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        let hits = self.detect(&words);

        let mut out_words: Vec<String> = Vec::with_capacity(words.len());
        let mut skip_next_false_start = false;
        for (i, word) in words.iter().enumerate() {
            if skip_next_false_start {
                skip_next_false_start = false;
                // false start removes the short prefix token; keep the continuation
            }

            let word_hits: Vec<&DisfluencyHit> = hits
                .iter()
                .filter(|h| h.text == *word || (h.rule_id == "false_start" && h.text == *word))
                .collect();

            let mut drop = false;
            let mut replacement: Option<String> = None;
            for hit in word_hits {
                match hit.artifact_type {
                    ArtifactType::Filler | ArtifactType::OrphanLetter => drop = true,
                    ArtifactType::GluedOnset => {
                        if let Some(fixed) = patterns::collapse_glued_onset(word) {
                            replacement = Some(fixed);
                        }
                    }
                    ArtifactType::Stutter | ArtifactType::RepeatedWord => {
                        // Keep first syllable / first token of hyphenated repeat.
                        let first = word
                            .split(|c: char| c == '-' || c.is_whitespace())
                            .find(|s| !s.is_empty())
                            .unwrap_or(word);
                        replacement = Some(first.to_string());
                    }
                    ArtifactType::FalseStart => {
                        drop = true;
                        skip_next_false_start = false; // continuation stays
                    }
                    ArtifactType::EmptyHesitation => {
                        // Handled as filler on middle token; no-op here.
                    }
                }
            }

            // Empty hesitation: drop middle filler when window matches.
            if i > 0 && i + 1 < words.len() {
                let window = [words[i - 1], words[i], words[i + 1]];
                if self.detector.is_empty_hesitation(&window).is_some()
                    && self
                        .mode
                        .allows(Confidence::Certain)
                    && self.detector.is_filler(words[i]).is_some()
                {
                    drop = true;
                }
            }

            if drop {
                continue;
            }
            out_words.push(replacement.unwrap_or_else(|| (*word).to_string()));
        }

        let fixed = out_words.join(" ");
        (fixed, hits)
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
    fn glued_onset_fixed_in_light_mode() {
        let fixer = DisfluencyFixer::new("ru", Mode::Light);
        let (out, hits) = fixer.fix_text("Ччисто никаких ошибок");
        assert!(hits.iter().any(|h| h.rule_id == "glued_onset"));
        assert!(out.starts_with("Чисто "), "got: {out}");
    }

    #[test]
    fn ddavay_collapsed() {
        let fixer = DisfluencyFixer::new("ru", Mode::Light);
        let (out, _) = fixer.fix_text("Ддавай попробуем");
        assert_eq!(out, "Давай попробуем");
    }
}
