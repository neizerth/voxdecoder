//! Disfluency cleanup — fillers, orphan letters, stuttering (ADR 012 + ADR 014).
//!
//! Deterministic removal of speech artifacts without language models.

pub mod dictionary;
pub mod patterns;
pub mod detector;
pub mod fixer;

pub use dictionary::DisfluencyDictionary;
pub use detector::{DisfluencyDetector, DisfluencyHit, ArtifactType};
pub use fixer::{DisfluencyFixer, Mode};
