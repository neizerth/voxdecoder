//! Platform paths: config file and platform model cache.

use std::env;
use std::path::PathBuf;

use directories::ProjectDirs;

const ENV_CONFIG: &str = "VD_FIX_CASING_CONFIG";
const ENV_MODELS: &str = "VD_FIX_CASING_MODELS_DIR";

/// Resolved config.toml path (`VD_FIX_CASING_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    if let Ok(p) = env::var(ENV_CONFIG) {
        return PathBuf::from(p);
    }
    project_dirs()
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("vd-fix-casing-config.toml"))
}

/// Default packs / models directory (platform cache).
///
/// - Linux: `~/.cache/vd-fix-casing/models` (or `$XDG_CACHE_HOME/vd-fix-casing/models`)
/// - macOS: `~/Library/Caches/vd-fix-casing/models`
/// - Windows: `%LOCALAPPDATA%\vd-fix-casing\cache\models`
pub fn default_models_dir() -> PathBuf {
    if let Ok(p) = env::var(ENV_MODELS) {
        return PathBuf::from(p);
    }
    project_dirs()
        .map(|d| d.cache_dir().join("models"))
        .unwrap_or_else(|| PathBuf::from("models"))
}

/// CLI override > env > config `download_root` > platform cache.
pub fn resolve_models_dir(cli_override: Option<PathBuf>) -> PathBuf {
    if let Some(p) = cli_override {
        return p;
    }
    if env::var_os(ENV_MODELS).is_some() {
        return default_models_dir();
    }
    match crate::config::load(&config_path()) {
        Ok(cfg) => cfg
            .download_root
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(default_models_dir),
        Err(_) => default_models_dir(),
    }
}

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "vd-fix-casing")
}
