//! Terminology fixer — this binary only.

mod backend;
mod config;
mod fixer;

pub use config::TermsLoadOptions;
pub use fixer::{TermsError, TermsFixer};
