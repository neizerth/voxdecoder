//! ASR wording fixer — this binary only.

mod config;
mod context_fuzzy;
mod fixer;
pub mod lang;
pub mod report;
pub mod rule;
pub mod stages;

pub use config::AsrLoadOptions;
pub use fixer::{AsrError, AsrFixer};
