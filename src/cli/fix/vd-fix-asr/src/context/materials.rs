//! Load `--context` from a `vd-assets` bundle (`./assets`) or text sources.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Read-only materials from `--context`. Never mutated after load.
#[derive(Debug, Clone, Default)]
pub struct Materials {
    pub vocabulary: BTreeSet<String>,
    pub forms: BTreeSet<String>,
    pub source_paths: Vec<PathBuf>,
}

/// Prefer `--context ./assets` (vd-assets output). Also accepts text/Markdown paths.
pub fn load_materials(paths: &[PathBuf]) -> Result<Materials, String> {
    if paths.is_empty() {
        return Ok(Materials::default());
    }
    let dict = vd_assets::load_dictionary(paths, &vd_assets::DictionaryOptions::default())
        .map_err(|e| e.to_string())?;
    Ok(Materials {
        vocabulary: dict.vocabulary(),
        forms: dict.forms,
        source_paths: dict.source_paths,
    })
}
