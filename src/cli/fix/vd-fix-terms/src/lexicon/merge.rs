//! Merge term entries into a map (**last wins**).

use std::collections::HashMap;

use super::normalize_key;

/// One glossary record: canonical form + variants that must rewrite to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermEntry {
    pub canonical: String,
    pub variants: Vec<String>,
}

pub fn apply_entries(map: &mut HashMap<String, String>, entries: Vec<TermEntry>) {
    for entry in entries {
        let canonical = entry.canonical.trim();
        if canonical.is_empty() {
            continue;
        }
        for variant in entry.variants {
            let key = normalize_key(&variant);
            if key.is_empty() {
                continue;
            }
            // Do not map a key equal to the canonical's normalized form unless
            // the surface form differs — still store so lookups work.
            map.insert(key, canonical.to_string());
        }
        // Also accept the canonical spelling itself as already-correct (no-op rewrite).
        let canon_key = normalize_key(canonical);
        map.entry(canon_key)
            .or_insert_with(|| canonical.to_string());
    }
}
