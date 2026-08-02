//! Load options for the layout fixer.

use std::path::PathBuf;

use crate::types::{Language, ParagraphDensity, TimeMap};

#[derive(Debug, Clone)]
pub struct LayoutLoadOptions {
    pub language: Language,
    pub models_dir: PathBuf,
    pub density: ParagraphDensity,
    pub use_timemap: bool,
    pub timemap: Option<TimeMap>,
}
