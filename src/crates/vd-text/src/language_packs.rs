//! Language-specific rules and data for text cleanup (ADR 0013).
//!
//! Provides layered language packs: builtin → language (ru, en) → future extensions.

use crate::rule_engine::{Action, Condition, Rule, RuleEngine};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LanguagePackError {
    #[error("Unknown language: {0}")]
    UnknownLanguage(String),
}

/// Language pack identifier.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Language {
    Builtin,
    Russian,
    English,
}

impl std::str::FromStr for Language {
    type Err = LanguagePackError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "builtin" => Ok(Language::Builtin),
            "ru" | "russian" => Ok(Language::Russian),
            "en" | "english" => Ok(Language::English),
            other => Err(LanguagePackError::UnknownLanguage(other.to_string())),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Language::Builtin => write!(f, "builtin"),
            Language::Russian => write!(f, "ru"),
            Language::English => write!(f, "en"),
        }
    }
}

/// Language pack provider.
pub struct LanguagePack;

impl LanguagePack {
    /// Load builtin rules.
    pub fn builtin() -> RuleEngine {
        let mut engine = RuleEngine::empty();

        // Punctuation rules
        engine.add_rule(Rule {
            id: "double-period".to_string(),
            confidence: "certain".to_string(),
            when: Condition::Regex {
                pattern: r"\.\.+".to_string(),
            },
            action: Action::Replace {
                with: ".".to_string(),
            },
        });

        engine
    }

    /// Load Russian language pack (inherits builtin).
    pub fn russian() -> RuleEngine {
        let mut engine = Self::builtin();

        // Russian fillers (filler words)
        let ru_fillers = [
            "эээ", "ммм", "ээ", "угу", "ага", "типа", "как бы", "вроде",
        ];

        for filler in &ru_fillers {
            engine.add_rule(Rule {
                id: format!("ru-filler-{}", filler),
                confidence: "certain".to_string(),
                when: Condition::Token {
                    value: filler.to_string(),
                },
                action: Action::Remove,
            });
        }

        // Russian discourse markers (keep these for layout, but mark as optional)
        let ru_markers = vec!["однако", "однак"];

        for marker in ru_markers {
            engine.add_rule(Rule {
                id: format!("ru-discourse-{}", marker),
                confidence: "likely".to_string(),
                when: Condition::Token {
                    value: marker.to_string(),
                },
                action: Action::Remove,
            });
        }

        engine
    }

    /// Load English language pack (inherits builtin).
    pub fn english() -> RuleEngine {
        let mut engine = Self::builtin();

        // English fillers
        let en_fillers = [
            "um", "uh", "uhh", "umm", "ummm", "like", "you know", "basically",
        ];

        for filler in &en_fillers {
            engine.add_rule(Rule {
                id: format!("en-filler-{}", filler),
                confidence: "certain".to_string(),
                when: Condition::Token {
                    value: filler.to_string(),
                },
                action: Action::Remove,
            });
        }

        engine
    }

    /// Load language pack by name.
    pub fn load(lang: Language) -> RuleEngine {
        match lang {
            Language::Builtin => Self::builtin(),
            Language::Russian => Self::russian(),
            Language::English => Self::english(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_parse() {
        assert_eq!("builtin".parse::<Language>().unwrap(), Language::Builtin);
        assert_eq!("ru".parse::<Language>().unwrap(), Language::Russian);
        assert_eq!("russian".parse::<Language>().unwrap(), Language::Russian);
        assert_eq!("en".parse::<Language>().unwrap(), Language::English);
        assert_eq!("english".parse::<Language>().unwrap(), Language::English);
    }

    #[test]
    fn test_builtin_pack() {
        let engine = LanguagePack::builtin();
        assert!(!engine.rules().is_empty());
    }

    #[test]
    fn test_russian_pack() {
        let engine = LanguagePack::russian();
        // Should have builtin + Russian rules
        assert!(engine.rules().len() >= 8); // at least 8 filler rules
    }

    #[test]
    fn test_english_pack() {
        let engine = LanguagePack::english();
        // Should have builtin + English rules
        assert!(engine.rules().len() >= 8); // at least 8 filler rules
    }

    #[test]
    fn test_load_by_language() {
        let _builtin = LanguagePack::load(Language::Builtin);
        let _russian = LanguagePack::load(Language::Russian);
        let _english = LanguagePack::load(Language::English);
    }
}
