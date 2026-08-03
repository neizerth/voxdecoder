//! Disfluency cleanup — this binary only (ADR 0012 §1).
//!
//! Core rule: remove speech noise, never remove information.

mod fixer;
pub mod rules;

pub use fixer::{DisfluencyError, DisfluencyFixer, DisfluencyLoadOptions};
pub use rules::Mode;
