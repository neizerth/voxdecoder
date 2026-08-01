//! Shared domain types for transcript artifacts and fix CLIs.

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

/// Opaque span identity for neighbor lookup, progress, logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(pub u32);

/// Per-fix knobs. Keep thin; prefer load-time options in each binary.
#[derive(Debug, Clone, Default)]
pub struct FixOptions {}

/// Only handle a fixer may mutate. Structure fields are unreachable.
pub struct TextSpan<'a> {
    pub id: SpanId,
    pub index: usize,
    pub text: &'a mut String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixResult {
    pub text: String,
    pub changed: bool,
}
