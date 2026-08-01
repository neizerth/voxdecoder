//! Load options for the presentation fixer.

use std::path::PathBuf;

use crate::types::Language;

#[derive(Debug, Clone)]
pub struct CasingLoadOptions {
    pub language: Language,
    pub models_dir: PathBuf,
}
