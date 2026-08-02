//! Domain types — re-export shared crates + layout-local resolved config.

pub use vd_artifact::{
    ArtifactType, FixOptions, FixResult, Language, SpanId, TextSpan, TimeMap,
};
pub use vd_progress::ProgressFormat;

/// Paragraph density policy (public). Thresholds live in the language pack / backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphDensity {
    Compact,
    Normal,
    Relaxed,
}

impl ParagraphDensity {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "compact" => Some(Self::Compact),
            "normal" => Some(Self::Normal),
            "relaxed" => Some(Self::Relaxed),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Normal => "normal",
            Self::Relaxed => "relaxed",
        }
    }

    pub fn allowed() -> &'static [&'static str] {
        &["compact", "normal", "relaxed"]
    }

    /// Target sentences per paragraph (soft).
    pub fn target_sentences(self) -> usize {
        match self {
            Self::Compact => 6,
            Self::Normal => 4,
            Self::Relaxed => 2,
        }
    }

    /// Hard max before forcing a break.
    pub fn max_sentences(self) -> usize {
        match self {
            Self::Compact => 10,
            Self::Normal => 6,
            Self::Relaxed => 4,
        }
    }
}

/// Abstract TimeMap binding — not a promised filesystem path in the product contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeMapSource {
    Artifact,
    Job,
    Runtime,
    Cli,
    None,
}

impl TimeMapSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Job => "job",
            Self::Runtime => "runtime",
            Self::Cli => "cli",
            Self::None => "none",
        }
    }
}

/// Resolved settings after CLI > config > default merge (layout-specific).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub language: Language,
    pub density: ParagraphDensity,
    pub use_timemap: bool,
    pub in_place: bool,
    pub progress: ProgressFormat,
}
