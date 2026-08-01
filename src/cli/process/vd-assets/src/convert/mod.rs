//! Convert Office/PDF → Markdown and build `terms.yml` project assets.

mod cache;
mod extract;

use std::fs;
use std::path::{Path, PathBuf};

use crate::dict::{self, write_terms, Dictionary, DictionaryOptions};
use crate::paths;

pub use extract::{ExtractError, ExtractOptions, OcrMode};

#[derive(Debug, Clone)]
pub struct ConvertRequest {
    pub inputs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub ocr: OcrMode,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub markdown_dir: PathBuf,
    pub terms_path: PathBuf,
    pub converted: Vec<PathBuf>,
    pub text_sources: Vec<PathBuf>,
    pub dictionary: Dictionary,
}

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("{0}")]
    Extract(#[from] ExtractError),
    #[error("{0}")]
    Dict(#[from] dict::DictionaryError),
    #[error("{0}")]
    Other(String),
}

impl ConvertError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Extract(e) => e.exit_code(),
            Self::Dict(e) => e.exit_code(),
            Self::Other(_) => 1,
        }
    }
}

/// Prepare project assets: convert Office/PDF when needed, then build `terms.yml`.
///
/// Conversion is **required** when none of the inputs are already text/Markdown.
pub fn run(req: &ConvertRequest) -> Result<ConvertResult, ConvertError> {
    let files = collect_files(&req.inputs)?;
    if files.is_empty() {
        return Err(ConvertError::Other(
            "no input files found under -i paths".into(),
        ));
    }

    let textish: Vec<_> = files
        .iter()
        .filter(|p| dict::is_text_source(p))
        .cloned()
        .collect();
    let convertible: Vec<_> = files
        .iter()
        .filter(|p| dict::is_office_or_pdf(p))
        .cloned()
        .collect();

    if textish.is_empty() && convertible.is_empty() {
        return Err(ConvertError::Other(
            "no text/Markdown or Office/PDF sources found".into(),
        ));
    }

    let must_convert = textish.is_empty();
    if must_convert && convertible.is_empty() {
        return Err(ConvertError::Other(
            "no text/Markdown among inputs; Office/PDF conversion required but nothing convertible found"
                .into(),
        ));
    }

    let md_dir = req.output_dir.join("md");
    fs::create_dir_all(&md_dir).map_err(|e| ConvertError::Other(e.to_string()))?;

    let cache = cache::CacheStore::new(paths::cache_root());
    let extract_opts = ExtractOptions {
        ocr: req.ocr,
        cache,
        force: req.force,
    };

    let mut converted = Vec::new();
    let mut processed_texts: Vec<PathBuf> = Vec::new();

    // Always convert convertible docs → md (required when no text sources).
    for src in &convertible {
        let doc = extract::resolve_document(src, &extract_opts)?;
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        let out_md = unique_md_path(&md_dir, stem);
        let body = format!(
            "<!-- source: {} -->\n\n{}\n",
            src.display(),
            doc.text.trim()
        );
        fs::write(&out_md, body).map_err(|e| ConvertError::Other(e.to_string()))?;
        converted.push(out_md.clone());
        processed_texts.push(out_md);
    }

    if must_convert && converted.is_empty() {
        return Err(ConvertError::Other(
            "conversion required (no text/Markdown inputs) but produced no Markdown".into(),
        ));
    }

    // Include original text/md sources in the dictionary build (and copy into md/).
    for src in &textish {
        let name = src
            .file_name()
            .map(std::ffi::OsStr::to_owned)
            .unwrap_or_else(|| "source.txt".into());
        let mut dest = md_dir.join(&name);
        if dest.exists() {
            let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("src");
            let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("txt");
            dest = unique_path(&md_dir, &format!("{stem}.orig"), ext);
        }
        fs::copy(src, &dest).map_err(|e| ConvertError::Other(e.to_string()))?;
        processed_texts.push(dest);
    }

    let dict = dict::load_dictionary(&processed_texts, &DictionaryOptions::default())?;
    let terms_path = write_terms(&req.output_dir, &dict)?;

    Ok(ConvertResult {
        markdown_dir: md_dir,
        terms_path,
        converted,
        text_sources: textish,
        dictionary: dict,
    })
}

fn collect_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, ConvertError> {
    let mut out = Vec::new();
    for path in inputs {
        if !path.exists() {
            return Err(ConvertError::Other(format!(
                "input missing: {}",
                path.display()
            )));
        }
        collect_one(path, &mut out)?;
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn collect_one(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), ConvertError> {
    if path.is_file() {
        if dict::is_text_source(path) || dict::is_office_or_pdf(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| ConvertError::Other(e.to_string()))? {
            let entry = entry.map_err(|e| ConvertError::Other(e.to_string()))?;
            collect_one(&entry.path(), out)?;
        }
    }
    Ok(())
}

fn unique_md_path(dir: &Path, stem: &str) -> PathBuf {
    unique_path(dir, stem, "md")
}

fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{stem}.{ext}"));
    let mut n = 2u32;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}-{n}.{ext}"));
        n += 1;
    }
    candidate
}
