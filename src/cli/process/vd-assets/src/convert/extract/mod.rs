//! Document → UTF-8 text extractors (text / office / pdf + optional OCR).

mod docx;
mod ocr;
mod pdf;
mod plain;
mod xlsx;

use std::path::{Path, PathBuf};

use crate::convert::cache::CacheStore;

/// Whether to fall back to local `tesseract` OCR when native text is thin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OcrMode {
    #[default]
    Off,
    /// Run OCR when extracted text is empty / below threshold.
    Auto,
    /// Always try OCR after (or instead of) native extract when available.
    On,
}

impl OcrMode {
    pub fn enabled(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn from_flag(ocr: bool) -> Self {
        if ocr {
            Self::Auto
        } else {
            Self::Off
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub ocr: OcrMode,
    pub cache: CacheStore,
    /// Rebuild even if a valid cache entry exists.
    pub force: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            ocr: OcrMode::Off,
            cache: CacheStore::default_store(),
            force: false,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExtractedDocument {
    pub source: PathBuf,
    pub text: String,
    pub from_cache: bool,
    pub cache_text_path: PathBuf,
    pub cache_dict_path: PathBuf,
    pub used_ocr: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("path missing / unreadable: {0}")]
    Missing(String),
    #[error("unsupported document type: {0}")]
    Unsupported(String),
    #[error("failed to extract text: {0}")]
    Failed(String),
    #[error("cache error: {0}")]
    Cache(String),
}

impl ExtractError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Missing(_) | Self::Unsupported(_) => 3,
            Self::Failed(_) | Self::Cache(_) => 1,
        }
    }
}

/// Resolve UTF-8 text for a file, using cache when fingerprint matches.
pub fn resolve_document(
    path: &Path,
    opts: &ExtractOptions,
) -> Result<ExtractedDocument, ExtractError> {
    if !path.exists() {
        return Err(ExtractError::Missing(path.display().to_string()));
    }
    if path.is_dir() {
        return Err(ExtractError::Unsupported(
            "resolve_document expects a file; use load_dictionary for directories".into(),
        ));
    }

    if !opts.force {
        if let Some(hit) = opts.cache.get(path, opts.ocr) {
            return Ok(ExtractedDocument {
                source: path.to_path_buf(),
                text: hit.text,
                from_cache: true,
                cache_text_path: hit.text_path,
                cache_dict_path: hit.dict_path,
                used_ocr: false,
            });
        }
    }

    let (mut text, mut used_ocr) = extract_native(path)?;
    let thin = text.chars().filter(|c| !c.is_whitespace()).count() < 40;
    if opts.ocr.enabled() && (matches!(opts.ocr, OcrMode::On) || thin) {
        if let Ok(ocr_text) = ocr::run_tesseract(path) {
            let ocr_len = ocr_text.chars().filter(|c| !c.is_whitespace()).count();
            let native_len = text.chars().filter(|c| !c.is_whitespace()).count();
            if ocr_len > native_len {
                text = ocr_text;
                used_ocr = true;
            }
        }
    }

    let hit = opts
        .cache
        .put(path, opts.ocr, &text, None)
        .map_err(ExtractError::Cache)?;

    Ok(ExtractedDocument {
        source: path.to_path_buf(),
        text: hit.text,
        from_cache: false,
        cache_text_path: hit.text_path,
        cache_dict_path: hit.dict_path,
        used_ocr,
    })
}

fn extract_native(path: &Path) -> Result<(String, bool), ExtractError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let text = match ext.as_str() {
        "txt" | "md" | "markdown" | "rst" | "json" | "yaml" | "yml" | "toml" | "csv" | "tsv"
        | "html" | "htm" | "xml" | "rs" | "py" | "ts" | "js" | "go" | "java" | "c" | "h"
        | "cpp" | "hpp" | "cs" | "sh" | "bash" | "zsh" | "env" | "ini" | "cfg" | "conf" => {
            plain::read_utf8(path).map_err(ExtractError::Failed)?
        }
        "pdf" => pdf::extract(path).map_err(ExtractError::Failed)?,
        "docx" => docx::extract(path).map_err(ExtractError::Failed)?,
        "doc" => docx::extract_legacy_doc(path).map_err(ExtractError::Failed)?,
        "xlsx" | "xlsm" | "xls" => xlsx::extract(path).map_err(ExtractError::Failed)?,
        "" => {
            // extensionless README-like
            plain::read_utf8(path).map_err(ExtractError::Failed)?
        }
        other => {
            // try utf-8 plain; else unsupported
            match plain::read_utf8(path) {
                Ok(t) => t,
                Err(_) => {
                    return Err(ExtractError::Unsupported(format!(
                        "{} ({other})",
                        path.display()
                    )))
                }
            }
        }
    };
    Ok((text, false))
}
