//! Domain types — re-export shared crates + casing-local resolved config.

pub use vd_artifact::{
    ArtifactType, FixOptions, FixResult, Language, SpanId, TextSpan,
};
pub use vd_progress::ProgressFormat;

/// Resolved settings after CLI > config > default merge (casing-specific).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub language: Language,
    pub in_place: bool,
    pub progress: ProgressFormat,
}
