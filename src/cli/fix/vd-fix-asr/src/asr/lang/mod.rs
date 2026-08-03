//! Language packs (ADR 0010).
//!
//! Builtin ASR-mistake dictionary layered with project/user dictionaries —
//! `builtin → pack → project → user`, last layer wins. No shipped "pack"
//! assets exist yet beyond the builtin table; that layer is a placeholder
//! until language packs beyond `ru`/`en` ship.

mod en;
mod ru;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::types::Language;

fn insert_all(map: &mut HashMap<String, String>, pairs: &[(&str, &str)]) {
    for (from, to) in pairs {
        map.insert((*from).to_string(), (*to).to_string());
    }
}

fn builtin_table(language: Language) -> &'static [(&'static str, &'static str)] {
    match language {
        Language::En => en::BUILTIN,
        _ => ru::BUILTIN, // ru / de / auto → ru-priority path, same default as the rest of this crate
    }
}

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

/// Merges builtin → pack → project → user (last wins) into a single
/// `variant(lowercase) → canonical` replacement map for Stage 6's static
/// lookup rule.
///
/// `dictionary_paths` / `project_dir` are empty/`None` until the
/// `--dictionary`/`--project` CLI flags land (later PR).
pub fn resolve_dictionary(
    language: Language,
    dictionary_paths: &[PathBuf],
    project_dir: Option<&Path>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    insert_all(&mut map, builtin_table(language));
    // pack layer intentionally absent — see module docs.
    if let Some(dir) = project_dir {
        let project_dict = dir.join(".voxdecoder").join("asr-dictionary.yml");
        if project_dict.is_file() {
            map.extend(load_layer(&[project_dict]));
        }
    }
    map.extend(load_layer(dictionary_paths));
    map
}
