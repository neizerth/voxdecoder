//! Domain types — re-export shared crates plus this crate's `Mode`.

pub use vd_artifact::{ArtifactType, FixOptions, FixResult, Language, SpanId, TextSpan};
pub use vd_progress::ProgressFormat;

pub use crate::disfluency::Mode;
