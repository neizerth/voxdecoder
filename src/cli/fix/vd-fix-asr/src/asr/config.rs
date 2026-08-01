//! Load-time options for `AsrFixer`.

use std::path::PathBuf;

use crate::types::Language;

#[derive(Debug, Clone)]
pub struct AsrLoadOptions {
    pub language: Language,
    /// Paths from repeatable `--context`.
    pub context_paths: Vec<PathBuf>,
    /// Neighboring segments window (`--context-neighbors`).
    pub neighbor_window: u32,
}
