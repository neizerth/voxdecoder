//! Terms / project-asset load for prepared `vd-assets` output (and plain text glossaries).
//!
//! Does **not** convert Office/PDF — that is `vd-assets run`. Fix CLIs pass an
//! assets directory (`md/` + `terms.yml`) or ready text sources.

mod glossary;
mod tokenize;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use glossary::TermEntry;

/// On-disk terms file produced by `vd-assets` inside an assets directory.
/// Shared format for `vd-fix-asr` / `vd-fix-terms`.
pub const TERMS_NAME: &str = "terms.yml";
/// Markdown subdirectory inside an assets directory.
pub const MD_DIR: &str = "md";
/// Legacy names still accepted when reading.
const LEGACY_TERMS_NAMES: &[&str] = &["terms.yaml", "lexicon.yaml", "dictionary.yaml"];

#[derive(Debug, Clone, Default)]
pub struct Dictionary {
    pub forms: BTreeSet<String>,
    pub entries: Vec<TermEntry>,
    pub source_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct DictionaryOptions {
    pub max_files: usize,
}

impl Default for DictionaryOptions {
    fn default() -> Self {
        Self { max_files: 500 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error("path missing / unreadable: {0}")]
    Missing(String),
    #[error("{0}")]
    Other(String),
}

impl DictionaryError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Missing(_) => 3,
            Self::Other(_) => 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TermsFile {
    version: u32,
    #[serde(default)]
    entries: Vec<TermEntry>,
    #[serde(default)]
    forms: Vec<String>,
}

/// True when `path` looks like a `vd-assets` output bundle (`terms.yml` and/or `md/`).
pub fn is_assets_dir(path: &Path) -> bool {
    path.is_dir()
        && (path.join(TERMS_NAME).is_file()
            || LEGACY_TERMS_NAMES
                .iter()
                .any(|n| path.join(n).is_file())
            || path.join(MD_DIR).is_dir())
}

/// Load project knowledge from assets dirs and/or text / Markdown / glossary files.
///
/// For an assets directory, only `terms.yml` (or legacy names) and `md/`
/// are read — other subfolders are ignored so the bundle can grow later.
///
/// Office/PDF paths are rejected — run `vd-assets` first.
pub fn load_dictionary(
    paths: &[PathBuf],
    opts: &DictionaryOptions,
) -> Result<Dictionary, DictionaryError> {
    let mut dict = Dictionary::default();
    let mut files_seen = 0usize;
    for path in paths {
        ingest_path(path, opts, &mut dict, &mut files_seen)?;
    }
    Ok(dict)
}

/// Write `terms.yml` into an assets output directory.
pub fn write_terms(out_dir: &Path, dict: &Dictionary) -> Result<PathBuf, DictionaryError> {
    fs::create_dir_all(out_dir).map_err(|e| DictionaryError::Other(e.to_string()))?;
    let path = out_dir.join(TERMS_NAME);
    let file = TermsFile {
        version: 1,
        entries: dict.entries.clone(),
        forms: dict.forms.iter().cloned().collect(),
    };
    let body = serde_yaml::to_string(&file).map_err(|e| DictionaryError::Other(e.to_string()))?;
    fs::write(&path, body).map_err(|e| DictionaryError::Other(e.to_string()))?;
    Ok(path)
}

/// Alias; writes [`TERMS_NAME`].
pub fn write_lexicon(out_dir: &Path, dict: &Dictionary) -> Result<PathBuf, DictionaryError> {
    write_terms(out_dir, dict)
}

/// Alias; writes [`TERMS_NAME`].
pub fn write_dictionary(out_dir: &Path, dict: &Dictionary) -> Result<PathBuf, DictionaryError> {
    write_terms(out_dir, dict)
}

/// True for UTF-8 project text sources (no Office/PDF).
pub fn is_text_source(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some(
            "txt" | "md" | "markdown" | "rst" | "json" | "yaml" | "yml" | "toml" | "csv" | "tsv"
            | "html" | "htm" | "xml" | "rs" | "py" | "ts" | "js" | "go" | "java" | "c" | "h"
            | "cpp" | "hpp" | "cs" | "sh" | "bash" | "zsh" | "env" | "ini" | "cfg" | "conf",
        ) => true,
        Some(_) => false,
        None => path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            let lower = n.to_ascii_lowercase();
            lower == "readme"
                || lower == "license"
                || lower == "makefile"
                || lower == "dockerfile"
                || lower == TERMS_NAME
                || LEGACY_TERMS_NAMES.contains(&lower.as_str())
                || lower.starts_with("readme.")
        }),
    }
}

fn ingest_path(
    path: &Path,
    opts: &DictionaryOptions,
    dict: &mut Dictionary,
    files_seen: &mut usize,
) -> Result<(), DictionaryError> {
    if !path.exists() {
        return Err(DictionaryError::Missing(path.display().to_string()));
    }
    if path.is_dir() {
        if is_assets_dir(path) {
            return ingest_assets_dir(path, opts, dict, files_seen);
        }
        let mut entries: Vec<_> = fs::read_dir(path)
            .map_err(|e| DictionaryError::Other(format!("{}: {e}", path.display())))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        entries.sort();
        for child in entries {
            if child.is_dir() {
                ingest_path(&child, opts, dict, files_seen)?;
            } else if is_text_source(&child) {
                ingest_file(&child, opts, dict, files_seen)?;
            } else if is_office_or_pdf(&child) {
                return Err(DictionaryError::Other(format!(
                    "{}: Office/PDF not loaded here — run `vd-assets run -i … -o <assets>` first",
                    child.display()
                )));
            }
        }
        return Ok(());
    }
    if is_office_or_pdf(path) {
        return Err(DictionaryError::Other(format!(
            "{}: Office/PDF not loaded here — run `vd-assets run -i … -o <assets>` first",
            path.display()
        )));
    }
    ingest_file(path, opts, dict, files_seen)
}

/// Load only the stable assets layout: terms file + `md/`.
fn ingest_assets_dir(
    path: &Path,
    opts: &DictionaryOptions,
    dict: &mut Dictionary,
    files_seen: &mut usize,
) -> Result<(), DictionaryError> {
    let terms = path.join(TERMS_NAME);
    if terms.is_file() {
        ingest_file(&terms, opts, dict, files_seen)?;
    } else {
        for name in LEGACY_TERMS_NAMES {
            let legacy = path.join(name);
            if legacy.is_file() {
                ingest_file(&legacy, opts, dict, files_seen)?;
                break;
            }
        }
    }

    let md = path.join(MD_DIR);
    if md.is_dir() {
        ingest_path(&md, opts, dict, files_seen)?;
    }

    if dict.source_paths.is_empty() {
        return Err(DictionaryError::Other(format!(
            "{}: assets dir has no {TERMS_NAME} or {MD_DIR}/ — run `vd-assets` first",
            path.display()
        )));
    }
    Ok(())
}

fn ingest_file(
    path: &Path,
    opts: &DictionaryOptions,
    dict: &mut Dictionary,
    files_seen: &mut usize,
) -> Result<(), DictionaryError> {
    if *files_seen >= opts.max_files {
        return Ok(());
    }
    let text = fs::read_to_string(path)
        .map_err(|e| DictionaryError::Other(format!("{}: {e}", path.display())))?;
    *files_seen += 1;
    dict.source_paths.push(path.to_path_buf());

    if is_terms_file(path) {
        if let Ok(file) = serde_yaml::from_str::<TermsFile>(&text) {
            dict.entries.extend(file.entries);
            dict.forms.extend(file.forms);
            return Ok(());
        }
    }

    let entries = glossary::parse_any(&text).unwrap_or_default();
    for e in &entries {
        dict.forms.insert(e.canonical.clone());
        for v in &e.variants {
            dict.forms.insert(v.clone());
        }
    }
    dict.entries.extend(entries);

    for token in tokenize::tokens(&text) {
        if token.chars().count() >= 2 {
            dict.forms.insert(token);
        }
    }
    Ok(())
}

fn is_terms_file(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        let lower = n.to_ascii_lowercase();
        lower == TERMS_NAME
            || LEGACY_TERMS_NAMES.contains(&lower.as_str())
            || lower.ends_with(".dict.yaml")
            || lower.ends_with(".dict.yml")
    })
}

pub fn is_office_or_pdf(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("pdf" | "docx" | "doc" | "xlsx" | "xlsm" | "xls")
    )
}

impl Dictionary {
    pub fn vocabulary(&self) -> BTreeSet<String> {
        self.forms.iter().map(|f| f.to_lowercase()).collect()
    }

    pub fn term_entries_for_fixer(&self) -> Vec<TermEntry> {
        let mut out = self.entries.clone();
        let mut seen: BTreeSet<String> = out
            .iter()
            .flat_map(|e| {
                std::iter::once(e.canonical.to_lowercase())
                    .chain(e.variants.iter().map(|v| v.to_lowercase()))
            })
            .collect();
        for form in &self.forms {
            let key = form.to_lowercase();
            if seen.contains(&key) {
                continue;
            }
            if looks_like_term(form) {
                seen.insert(key);
                out.push(TermEntry {
                    canonical: form.clone(),
                    variants: vec![form.to_lowercase()],
                });
            }
        }
        out
    }

    pub fn merge_from(&mut self, other: Self) {
        self.forms.extend(other.forms);
        self.entries.extend(other.entries);
        self.source_paths.extend(other.source_paths);
    }
}

fn looks_like_term(form: &str) -> bool {
    let chars: Vec<char> = form.chars().collect();
    if chars.len() < 3 {
        return false;
    }
    let has_upper = chars.iter().any(char::is_ascii_uppercase);
    let has_digit = chars.iter().any(char::is_ascii_digit);
    has_upper || has_digit || form.contains(['/', '+', '.'])
}
