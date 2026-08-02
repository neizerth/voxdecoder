//! Platform paths: config file and GigaAM model cache.

use std::env;
use std::path::PathBuf;

use directories::{BaseDirs, ProjectDirs};

const ENV_CONFIG: &str = "VD_GIGAAM_CONFIG";
const ENV_MODELS: &str = "VD_GIGAAM_MODELS_DIR";
const ENV_MODELS_ROOT: &str = "VD_MODELS_DIR";

/// Resolved config.toml path (`VD_GIGAAM_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    if let Ok(p) = env::var(ENV_CONFIG) {
        return PathBuf::from(p);
    }
    project_dirs()
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("vd-gigaam-config.toml"))
}

/// Default install / models root — same layout as Python GigaAM cache.
///
/// Resolution order:
/// 1. `VD_GIGAAM_MODELS_DIR`
/// 2. `$VD_MODELS_DIR/gigaam` (shared Runtime models root)
/// 3. platform cache (`~/.cache/gigaam`, …)
pub fn default_models_dir() -> PathBuf {
    if let Ok(p) = env::var(ENV_MODELS) {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    if let Ok(root) = env::var(ENV_MODELS_ROOT) {
        let t = root.trim();
        if !t.is_empty() {
            return PathBuf::from(t).join("gigaam");
        }
    }
    preferred_gigaam_cache()
}

/// Preferred Python-compatible GigaAM cache path (always resolved; may not exist yet).
pub fn preferred_gigaam_cache() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("gigaam");
        }
    }
    if let Some(base) = BaseDirs::new() {
        #[cfg(windows)]
        {
            return base.cache_dir().join("gigaam");
        }
        #[cfg(not(windows))]
        {
            // Python GigaAM hardcodes expanduser("~/.cache/gigaam") on Unix (incl. macOS).
            return base.home_dir().join(".cache").join("gigaam");
        }
    }
    PathBuf::from(".cache").join("gigaam")
}

/// Candidate dirs where a Python / prior install may have left `.ckpt` files.
pub fn gigaam_cache_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(preferred_gigaam_cache());
    if let Ok(xdg) = env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            candidates.push(PathBuf::from(xdg).join("gigaam"));
        }
    }
    if let Some(base) = BaseDirs::new() {
        candidates.push(base.home_dir().join(".cache").join("gigaam"));
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

/// First existing GigaAM cache directory among candidates (may be missing).
pub fn gigaam_cache_dir() -> Option<PathBuf> {
    gigaam_cache_candidates()
        .into_iter()
        .find(|p| p.is_dir())
}

/// CLI override > `VD_GIGAAM_MODELS_DIR` > config `download_root` > platform GigaAM cache.
pub fn resolve_models_dir(cli_override: Option<PathBuf>) -> PathBuf {
    if let Some(p) = cli_override {
        return p;
    }
    if env::var_os(ENV_MODELS).is_some() {
        return default_models_dir();
    }
    match crate::config::file::load(&config_path()) {
        Ok(cfg) => cfg
            .download_root
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(default_models_dir),
        Err(_) => default_models_dir(),
    }
}

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "vd-gigaam")
}
