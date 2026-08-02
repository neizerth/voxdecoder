//! Install / load language packs into the models directory.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::catalog::{self, CatalogEntry};
use crate::types::Language;

pub type ProgressFn<'a> = dyn FnMut(u64, Option<u64>) + 'a;

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("{0}")]
    Message(String),
    #[error("unknown model '{0}'")]
    Unknown(String),
    #[error("pack not shipping yet: {0}")]
    NotShipping(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl PackError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Unknown(_) | Self::NotShipping(_) => 2,
            Self::Message(_) | Self::Io(_) => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub language: String,
    pub version: u32,
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lexicon {
    /// Sentence-start discourse markers that suggest a paragraph break.
    pub discourse: Vec<String>,
    /// Soft connective cues (weaker break signal).
    pub soft_break: Vec<String>,
}

#[derive(Debug)]
pub enum InstallOutcome {
    AlreadyPresent(PathBuf),
    Installed(PathBuf),
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub name: String,
    pub language: String,
    pub shipping: bool,
    pub installed: bool,
    pub path: Option<String>,
    pub mark: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub language: String,
    pub backend: String,
    pub version: u32,
    pub installed: bool,
    pub path: Option<String>,
    pub size: Option<u64>,
}

pub fn pack_dir(models_root: &Path, name: &str) -> PathBuf {
    models_root.join(catalog::resolve_model_name(name))
}

pub fn is_installed(models_root: &Path, name: &str) -> bool {
    let dir = pack_dir(models_root, name);
    dir.join("manifest.toml").is_file() && dir.join("lexicon.json").is_file()
}

pub fn install(
    models_root: &Path,
    name: &str,
    force: bool,
    on_progress: Option<&mut ProgressFn<'_>>,
) -> Result<InstallOutcome, PackError> {
    let resolved = catalog::resolve_model_name(name).to_string();
    let entry = catalog::entry(&resolved).ok_or_else(|| PackError::Unknown(name.into()))?;
    if !entry.shipping {
        return Err(PackError::NotShipping(resolved));
    }

    fs::create_dir_all(models_root)?;
    scrub_tmps(models_root, &resolved);

    let dir = pack_dir(models_root, &resolved);
    if !force && is_installed(models_root, &resolved) {
        return Ok(InstallOutcome::AlreadyPresent(dir));
    }
    if force && dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }

    let (manifest, lexicon) = embedded_pack(entry)?;
    let total = (manifest.len() + lexicon.len()) as u64;
    let mut done = 0u64;
    let mut on_progress = on_progress;
    report(&mut on_progress, done, Some(total));

    fs::create_dir_all(&dir)?;
    let tmp_manifest = dir.join("manifest.toml.tmp");
    let tmp_lexicon = dir.join("lexicon.json.tmp");

    {
        let mut f = fs::File::create(&tmp_manifest)?;
        f.write_all(manifest.as_bytes())?;
        done += manifest.len() as u64;
        report(&mut on_progress, done, Some(total));
    }
    {
        let mut f = fs::File::create(&tmp_lexicon)?;
        f.write_all(lexicon.as_bytes())?;
        done += lexicon.len() as u64;
        report(&mut on_progress, done, Some(total));
    }

    fs::rename(&tmp_manifest, dir.join("manifest.toml"))?;
    fs::rename(&tmp_lexicon, dir.join("lexicon.json"))?;
    report(&mut on_progress, total, Some(total));

    Ok(InstallOutcome::Installed(dir))
}

pub fn remove(models_root: &Path, name: &str) -> Result<(), PackError> {
    let resolved = catalog::resolve_model_name(name).to_string();
    if catalog::entry(&resolved).is_none() {
        return Err(PackError::Unknown(name.into()));
    }
    let dir = pack_dir(models_root, &resolved);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

pub fn list_status(models_root: &Path, include_all: bool) -> Vec<ModelStatus> {
    catalog::CATALOG
        .iter()
        .filter(|e| include_all || e.shipping || is_installed(models_root, e.name))
        .map(|e| {
            let installed = is_installed(models_root, e.name);
            let mark = if installed {
                "✓"
            } else if e.shipping {
                "○"
            } else {
                "·"
            };
            ModelStatus {
                name: e.name.to_string(),
                language: e.language.as_str().to_string(),
                shipping: e.shipping,
                installed,
                path: installed.then(|| pack_dir(models_root, e.name).display().to_string()),
                mark: mark.to_string(),
            }
        })
        .collect()
}

pub fn info(models_root: &Path, name: &str) -> Result<ModelInfo, PackError> {
    let resolved = catalog::resolve_model_name(name).to_string();
    let entry = catalog::entry(&resolved).ok_or_else(|| PackError::Unknown(name.into()))?;
    let installed = is_installed(models_root, &resolved);
    let path = pack_dir(models_root, &resolved);
    let size = if installed {
        Some(dir_size(&path).unwrap_or(0))
    } else {
        None
    };
    let (backend, version) = if installed {
        match load_manifest(&path) {
            Ok(m) => (m.backend, m.version),
            Err(_) => (entry.backend.to_string(), entry.version),
        }
    } else {
        (entry.backend.to_string(), entry.version)
    };
    Ok(ModelInfo {
        name: resolved,
        language: entry.language.as_str().to_string(),
        backend,
        version,
        installed,
        path: installed.then(|| path.display().to_string()),
        size,
    })
}

/// Prefer an installed pack lexicon; otherwise use the embedded shipping lexicon.
pub fn resolve_lexicon(models_root: &Path, language: Language) -> Result<Lexicon, PackError> {
    let name = language_to_pack(language);
    if is_installed(models_root, name) {
        let path = pack_dir(models_root, name).join("lexicon.json");
        let text = fs::read_to_string(&path)?;
        return serde_json::from_str(&text)
            .map_err(|e| PackError::Message(format!("corrupt lexicon {}: {e}", path.display())));
    }
    Ok(builtin_lexicon(language))
}

pub fn builtin_lexicon(language: Language) -> Lexicon {
    match language {
        Language::En | Language::De => en_lexicon(),
        Language::Ru | Language::Auto => ru_lexicon(),
    }
}

fn language_to_pack(language: Language) -> &'static str {
    match language {
        Language::En | Language::De => "en",
        Language::Ru | Language::Auto => "ru",
    }
}

fn load_manifest(dir: &Path) -> Result<Manifest, PackError> {
    let text = fs::read_to_string(dir.join("manifest.toml"))?;
    toml::from_str(&text).map_err(|e| PackError::Message(e.to_string()))
}

fn embedded_pack(entry: &CatalogEntry) -> Result<(String, String), PackError> {
    let manifest = Manifest {
        name: entry.name.to_string(),
        language: entry.language.as_str().to_string(),
        version: entry.version,
        backend: entry.backend.to_string(),
    };
    let manifest_toml =
        toml::to_string_pretty(&manifest).map_err(|e| PackError::Message(e.to_string()))?;
    let lexicon = match entry.language {
        Language::En => en_lexicon(),
        _ => ru_lexicon(),
    };
    let lexicon_json =
        serde_json::to_string_pretty(&lexicon).map_err(|e| PackError::Message(e.to_string()))?;
    Ok((manifest_toml, lexicon_json))
}

fn ru_lexicon() -> Lexicon {
    Lexicon {
        discourse: vec![
            "ну",
            "вот",
            "значит",
            "короче",
            "типа",
            "слушай",
            "смотри",
            "кстати",
            "например",
            "вообще",
            "ладно",
            "итак",
            "далее",
            "кроме",
            "кроме того",
            "с другой стороны",
            "во-первых",
            "во-вторых",
            "в-третьих",
            "итак",
            "подводя",
            "резюмируя",
            "самое главное",
            "после",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        soft_break: vec!["а", "но", "или", "либо", "поэтому", "потом", "однако"]
            .into_iter()
            .map(str::to_string)
            .collect(),
    }
}

fn en_lexicon() -> Lexicon {
    Lexicon {
        discourse: vec![
            "well",
            "so",
            "like",
            "basically",
            "actually",
            "anyway",
            "okay",
            "ok",
            "right",
            "first",
            "second",
            "third",
            "finally",
            "meanwhile",
            "however",
            "moreover",
            "furthermore",
            "in addition",
            "on the other hand",
            "the most important",
            "after that",
            "next",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        soft_break: vec!["but", "or", "so", "then", "because", "however"]
            .into_iter()
            .map(str::to_string)
            .collect(),
    }
}

fn scrub_tmps(models_root: &Path, name: &str) {
    let dir = pack_dir(models_root, name);
    for name in ["manifest.toml.tmp", "lexicon.json.tmp"] {
        let p = dir.join(name);
        let _ = fs::remove_file(p);
    }
}

fn report(on_progress: &mut Option<&mut ProgressFn<'_>>, done: u64, total: Option<u64>) {
    if let Some(cb) = on_progress.as_mut() {
        cb(done, total);
    }
}

fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            total += meta.len();
        } else if meta.is_dir() {
            total += dir_size(&entry.path())?;
        }
    }
    Ok(total)
}
