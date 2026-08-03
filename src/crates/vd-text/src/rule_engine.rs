//! Declarative rule engine for text cleanup (ADR 0013).
//!
//! Rules are YAML-driven, language-specific, composed from conditions and actions.
//! Supports confidence levels (Certain/Likely/Unsafe) compatible with ADR 0010.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuleEngineError {
    #[error("Invalid confidence level: {0}")]
    InvalidConfidence(String),
    #[error("YAML parse error: {0}")]
    YamlError(String),
    #[error("Unknown action: {0}")]
    UnknownAction(String),
}

/// Confidence level for a rule (ADR 0010 compatible).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Certain,
    Likely,
    Unsafe,
}

impl std::str::FromStr for Confidence {
    type Err = RuleEngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "certain" => Ok(Confidence::Certain),
            "likely" => Ok(Confidence::Likely),
            "unsafe" => Ok(Confidence::Unsafe),
            other => Err(RuleEngineError::InvalidConfidence(other.to_string())),
        }
    }
}

/// Condition for triggering a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Match exact token
    Token { value: String },
    /// Match repeated word
    RepeatedWord {},
    /// Match merged word (no spaces)
    MergedWord {},
    /// Match regex pattern
    Regex { pattern: String },
}

/// Action to apply when condition matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Remove the matched text
    Remove,
    /// Remove the second occurrence (for duplicates)
    RemoveSecond,
    /// Split merged word
    Split,
    /// Replace with canonical form
    Replace { with: String },
}

/// A cleanup rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub confidence: String,
    pub when: Condition,
    pub action: Action,
}

impl Rule {
    /// Get confidence level.
    pub fn confidence(&self) -> Result<Confidence, RuleEngineError> {
        self.confidence.parse()
    }
}

/// Rule engine manages rule loading and application.
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    /// Create rule engine with rules.
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    /// Create empty rule engine.
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Load rules from YAML string.
    pub fn from_yaml(yaml_str: &str) -> Result<Self, RuleEngineError> {
        let rules: Vec<Rule> =
            serde_yaml::from_str(yaml_str).map_err(|e| RuleEngineError::YamlError(e.to_string()))?;
        Ok(Self::new(rules))
    }

    /// Get all rules.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Filter rules by confidence level.
    pub fn rules_for_confidence(&self, conf: Confidence) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|r| r.confidence().map(|c| c == conf).unwrap_or(false))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_parse() {
        assert_eq!("certain".parse::<Confidence>().unwrap(), Confidence::Certain);
        assert_eq!("likely".parse::<Confidence>().unwrap(), Confidence::Likely);
        assert_eq!("unsafe".parse::<Confidence>().unwrap(), Confidence::Unsafe);
    }

    #[test]
    fn test_rule_engine_empty() {
        let engine = RuleEngine::empty();
        assert_eq!(engine.rules().len(), 0);
    }

    #[test]
    fn test_rule_engine_add_rule() {
        let mut engine = RuleEngine::empty();
        let rule = Rule {
            id: "test".to_string(),
            confidence: "certain".to_string(),
            when: Condition::Token {
                value: "эээ".to_string(),
            },
            action: Action::Remove,
        };
        engine.add_rule(rule);
        assert_eq!(engine.rules().len(), 1);
    }

    #[test]
    fn test_rule_engine_from_yaml() {
        let yaml = r#"
- id: filler
  confidence: certain
  when:
    type: token
    value: эээ
  action:
    type: remove
"#;
        let engine = RuleEngine::from_yaml(yaml).unwrap();
        assert_eq!(engine.rules().len(), 1);
        assert_eq!(engine.rules()[0].id, "filler");
    }

    #[test]
    fn test_rule_engine_filter_by_confidence() {
        let mut engine = RuleEngine::empty();
        engine.add_rule(Rule {
            id: "certain-rule".to_string(),
            confidence: "certain".to_string(),
            when: Condition::Token {
                value: "эээ".to_string(),
            },
            action: Action::Remove,
        });
        engine.add_rule(Rule {
            id: "likely-rule".to_string(),
            confidence: "likely".to_string(),
            when: Condition::RepeatedWord {},
            action: Action::RemoveSecond,
        });

        let certain_rules = engine.rules_for_confidence(Confidence::Certain);
        assert_eq!(certain_rules.len(), 1);
        assert_eq!(certain_rules[0].id, "certain-rule");

        let likely_rules = engine.rules_for_confidence(Confidence::Likely);
        assert_eq!(likely_rules.len(), 1);
        assert_eq!(likely_rules[0].id, "likely-rule");
    }
}
