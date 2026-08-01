//! Load `--context` files / directories into a read-only vocabulary + text corpus.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 2_000_000;
const MAX_FILES: usize = 200;

/// Read-only materials from `--context`. Never mutated after load.
#[derive(Debug, Clone, Default)]
pub struct Materials {
    /// Lowercased tokens seen in context (recognition vocabulary hints).
    pub vocabulary: BTreeSet<String>,
    /// Original-cased tokens for preferred replacement forms.
    pub forms: BTreeSet<String>,
    pub source_paths: Vec<PathBuf>,
}

pub fn load_materials(paths: &[PathBuf]) -> Result<Materials, String> {
    let mut materials = Materials::default();
    let mut files_seen = 0usize;
    for path in paths {
        load_path(path, &mut materials, &mut files_seen)?;
    }
    Ok(materials)
}

fn load_path(path: &Path, materials: &mut Materials, files_seen: &mut usize) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("context path missing: {}", path.display()));
    }
    if path.is_file() {
        ingest_file(path, materials, files_seen)?;
        return Ok(());
    }
    if path.is_dir() {
        let entries = fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path();
            if p.is_dir() {
                load_path(&p, materials, files_seen)?;
            } else if is_textish(&p) {
                ingest_file(&p, materials, files_seen)?;
            }
        }
        return Ok(());
    }
    Ok(())
}

fn is_textish(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some(
            "txt" | "md" | "markdown" | "rst" | "json" | "yaml" | "yml" | "toml" | "rs" | "py"
            | "ts" | "js" | "go" | "java" | "kt" | "swift" | "c" | "h" | "cpp" | "hpp" | "cs"
            | "html" | "htm" | "css" | "xml" | "csv" | "tsv" | "sh" | "bash" | "zsh" | "env"
            | "ini" | "cfg" | "conf" | "gitignore" | "dockerfile",
        ) => true,
        Some(_) => false,
        None => {
            // extensionless: allow small files named README, LICENSE, Makefile, …
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| {
                    let lower = n.to_ascii_lowercase();
                    lower == "readme"
                        || lower == "license"
                        || lower == "makefile"
                        || lower == "dockerfile"
                        || lower.starts_with("readme.")
                })
        }
    }
}

fn ingest_file(
    path: &Path,
    materials: &mut Materials,
    files_seen: &mut usize,
) -> Result<(), String> {
    if *files_seen >= MAX_FILES {
        return Ok(());
    }
    let meta = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.len() > MAX_FILE_BYTES {
        return Ok(());
    }
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(()); // skip binary / undecodable
    };
    *files_seen += 1;
    materials.source_paths.push(path.to_path_buf());
    for token in tokenize(&text) {
        if token.chars().count() < 2 {
            continue;
        }
        materials.forms.insert(token.clone());
        materials.vocabulary.insert(token.to_lowercase());
    }
    Ok(())
}

fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '+'))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}
