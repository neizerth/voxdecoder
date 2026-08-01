//! Checkpoint cache / converted SafeTensors layout.

mod card;
mod download;

pub use card::ModelCard;
pub use download::{install_model, InstallOutcome};

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::gigaam::catalog::resolve_model_name;

#[derive(Debug, thiserror::Error)]
pub enum WeightsError {
    #[error("model weights not found: {0}")]
    NotFound(PathBuf),
    #[error("download not implemented yet for {0}")]
    DownloadNotImplemented(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("checksum mismatch for {path}: expected {expected}, got {got}")]
    Checksum {
        path: PathBuf,
        expected: String,
        got: String,
    },
    #[error("convert failed: {0}")]
    Convert(String),
    #[error("bad model card: {0}")]
    Card(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub dir: PathBuf,
    pub safetensors: PathBuf,
    pub card: PathBuf,
}

pub fn looks_like_path(s: &str) -> bool {
    s.contains('/')
        || s.contains('\\')
        || s.ends_with(".ckpt")
        || s.ends_with(".pt")
        || s.ends_with(".safetensors")
}

pub fn resolve_model_dir(download_root: &Path, model: &str) -> PathBuf {
    if looks_like_path(model) {
        let p = PathBuf::from(model);
        if p.is_dir() {
            return p;
        }
        if p.is_file() {
            return p.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        }
    }
    let name = crate::gigaam::catalog::resolve_model_name(model);
    download_root.join(name)
}

pub fn resolve_converted(download_root: &Path, model: &str) -> Result<ModelPaths, WeightsError> {
    let dir = resolve_model_dir(download_root, model);
    let safetensors = dir.join("model.safetensors");
    let card = dir.join("model.json");
    if safetensors.is_file() && card.is_file() {
        return Ok(ModelPaths {
            dir,
            safetensors,
            card,
        });
    }

    let name = if looks_like_path(model) {
        PathBuf::from(model)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(model)
            .to_string()
    } else {
        crate::gigaam::catalog::resolve_model_name(model).to_string()
    };
    let flat_st = download_root.join(format!("{name}.safetensors"));
    let flat_card = download_root.join(format!("{name}.json"));
    if flat_st.is_file() && flat_card.is_file() {
        return Ok(ModelPaths {
            dir: download_root.to_path_buf(),
            safetensors: flat_st,
            card: flat_card,
        });
    }

    Err(WeightsError::NotFound(safetensors))
}

pub fn checkpoint_path(download_root: &Path, model: &str) -> PathBuf {
    if looks_like_path(model) {
        return PathBuf::from(model);
    }
    let name = crate::gigaam::catalog::resolve_model_name(model);
    let converted = download_root.join(name).join("model.safetensors");
    if converted.is_file() {
        return converted;
    }
    download_root.join(format!("{name}.ckpt"))
}

pub fn is_installed(download_root: &Path, model: &str) -> bool {
    !matches!(model_status(download_root, model).kind, ModelKind::Missing)
}

/// Where catalog weights were found (for `list` / install reuse).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// SafeTensors + card under download_root — ready for `run`.
    Converted,
    /// `.ckpt` in download_root (not yet converted).
    ManagedCkpt,
    /// `.ckpt` only in Python GigaAM cache.
    GigaamCache,
    Missing,
}

#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub kind: ModelKind,
    pub path: Option<PathBuf>,
}

pub fn model_status(download_root: &Path, model: &str) -> ModelStatus {
    if let Ok(paths) = resolve_converted(download_root, model) {
        return ModelStatus {
            kind: ModelKind::Converted,
            path: Some(paths.safetensors),
        };
    }
    let name = resolve_model_name(model);
    let managed = download_root.join(format!("{name}.ckpt"));
    if managed.is_file() {
        return ModelStatus {
            kind: ModelKind::ManagedCkpt,
            path: Some(managed),
        };
    }
    if let Some(ext) = find_external_ckpt(name) {
        return ModelStatus {
            kind: ModelKind::GigaamCache,
            path: Some(ext),
        };
    }
    ModelStatus {
        kind: ModelKind::Missing,
        path: None,
    }
}

/// First existing Python GigaAM cache directory (may be empty / missing).
pub fn gigaam_cache_dir() -> Option<PathBuf> {
    gigaam_cache_candidates()
        .into_iter()
        .find(|p| p.is_dir())
}

/// Python GigaAM default cache (`~/.cache/gigaam`) — convenience only, not our install root.
pub fn find_external_ckpt(name: &str) -> Option<PathBuf> {
    let file = format!("{name}.ckpt");
    gigaam_cache_candidates()
        .into_iter()
        .map(|dir| dir.join(&file))
        .find(|p| p.is_file())
}

fn gigaam_cache_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    // Official Python GigaAM default (see checksum error paths in upstream docs).
    if let Ok(home) = env::var("HOME") {
        candidates.push(PathBuf::from(home).join(".cache/gigaam"));
    }
    if let Ok(xdg) = env::var("XDG_CACHE_HOME") {
        candidates.push(PathBuf::from(xdg).join("gigaam"));
    }
    if let Some(base) = BaseDirs::new() {
        candidates.push(base.cache_dir().join("gigaam"));
    }
    let mut out = Vec::new();
    for p in candidates {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

pub fn ensure_present(download_root: &Path, model: &str) -> Result<PathBuf, WeightsError> {
    if let Ok(paths) = resolve_converted(download_root, model) {
        return Ok(paths.safetensors);
    }
    let path = checkpoint_path(download_root, model);
    if path.is_file() {
        return Ok(path);
    }
    Err(WeightsError::NotFound(path))
}

pub fn install(
    download_root: &Path,
    model: &str,
    force: bool,
    on_progress: Option<&mut download::ProgressFn<'_>>,
) -> Result<InstallOutcome, WeightsError> {
    download::install_model(download_root, model, force, on_progress)
}

pub fn remove(download_root: &Path, model: &str) -> Result<(), WeightsError> {
    let name = crate::gigaam::catalog::resolve_model_name(model);
    let dir = download_root.join(name);
    if dir.is_dir() {
        fs::remove_dir_all(&dir)?;
    }
    let ckpt = download_root.join(format!("{name}.ckpt"));
    if ckpt.is_file() {
        fs::remove_file(&ckpt)?;
    }
    let tok = download_root.join(format!("{name}_tokenizer.model"));
    if tok.is_file() {
        fs::remove_file(&tok)?;
    }
    let st = download_root.join(format!("{name}.safetensors"));
    if st.is_file() {
        fs::remove_file(&st)?;
    }
    Ok(())
}
