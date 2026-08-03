//! Load-time options for `AsrFixer`.

use std::path::PathBuf;

use super::stages::ConfidencePolicy;
use crate::types::Language;

#[derive(Debug, Clone)]
pub struct AsrLoadOptions {
    pub language: Language,
    /// Paths from repeatable `--context`.
    pub context_paths: Vec<PathBuf>,
    /// Neighboring segments window (`--context-neighbors`).
    pub neighbor_window: u32,
    /// `--strict` / `--aggressive` (default: apply Certain + Likely).
    pub confidence_policy: ConfidencePolicy,
    /// Paths from repeatable `--dictionary` (highest-priority layer).
    pub dictionary_paths: Vec<PathBuf>,
    /// `--project` — provides `.voxdecoder/asr-dictionary.yml` if present.
    pub project_dir: Option<PathBuf>,
}
