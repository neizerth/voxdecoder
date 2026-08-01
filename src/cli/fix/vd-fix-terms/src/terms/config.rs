//! Load-time options for the terminology fixer.

use std::path::PathBuf;

use crate::types::Language;

/// Options passed to [`crate::lexicon::Lexicon::load`].
#[derive(Debug, Clone)]
pub struct TermsLoadOptions {
    pub language: Language,
    /// Include the shipping lexicon (default: true at the CLI).
    pub shipping: bool,
    /// Paths from repeatable `--terms` (left → right; **last wins** on conflict).
    pub terms_paths: Vec<PathBuf>,
}
