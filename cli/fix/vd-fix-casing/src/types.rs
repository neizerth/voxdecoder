//! Domain model for `vd-fix-casing`.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Txt,
    Json,
    Jsonl,
    Srt,
    Vtt,
    Md,
}

impl ArtifactType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Json => "json",
            Self::Jsonl => "jsonl",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Md => "md",
        }
    }

    pub fn extension(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Ru,
    En,
    De,
    Auto,
}

impl Language {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ru" => Some(Self::Ru),
            "en" => Some(Self::En),
            "de" => Some(Self::De),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
            Self::De => "de",
            Self::Auto => "auto",
        }
    }

    pub fn allowed() -> &'static [&'static str] {
        &["ru", "en", "de", "auto"]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressFormat {
    Text,
    Json,
}

impl ProgressFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

/// Resolved settings after CLI > config > default merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub language: Language,
    pub in_place: bool,
    pub progress: ProgressFormat,
}

/// Reserved options for `CasingFixer::fix` (keep thin).
#[derive(Debug, Clone, Default)]
pub struct FixOptions {}

/// Only handle the fixer may mutate.
pub struct TextSpan<'a> {
    pub text: &'a mut String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixResult {
    pub text: String,
    pub changed: bool,
}
