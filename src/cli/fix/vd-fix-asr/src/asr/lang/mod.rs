//! Language / dictionary resolution (ADR 0010).
//!
//! No hardcoded ASR-mistake table in the binary. Stage 6 dictionary comes
//! only from external layers (last wins):
//!
//! ```text
//! project   — {--project}/.voxdecoder/asr-dictionary.yml, if present
//! user      — --dictionary PATH (repeatable)
//! context   — meeting docs / materials feed fuzzy Stage 6 via Materials
//! ```
//!
//! Residual one-off mishears stay for optional AI cleanup — not for code tables.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::types::Language;

/// Loads one dictionary layer (`vd-assets`-style terms file or glossary) and
/// flattens it into `variant(lowercase) → canonical`. Missing/empty paths
/// are silently treated as an empty layer — the layer is optional by design.
fn load_layer(paths: &[PathBuf]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if paths.is_empty() {
        return map;
    }
    if let Ok(dict) = vd_assets::load_dictionary(paths, &vd_assets::DictionaryOptions::default()) {
        for entry in dict.term_entries_for_fixer() {
            for variant in &entry.variants {
                map.insert(variant.to_lowercase(), entry.canonical.clone());
            }
        }
    }
    map
}

/// Merges project → user (last wins) into a single
/// `variant(lowercase) → canonical` replacement map for Stage 6's static
/// lookup. Empty when neither source is provided.
pub fn resolve_dictionary(
    language: Language,
    dictionary_paths: &[PathBuf],
    project_dir: Option<&Path>,
) -> HashMap<String, String> {
    let _ = language; // reserved for future language-scoped pack *files*
    let mut map = HashMap::new();
    if let Some(dir) = project_dir {
        let project_dict = dir.join(".voxdecoder").join("asr-dictionary.yml");
        if project_dict.is_file() {
            map.extend(load_layer(&[project_dict]));
        }
    }
    map.extend(load_layer(dictionary_paths));
    map
}
