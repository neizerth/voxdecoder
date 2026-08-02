//! Platform paths: config file and managed model cache.

use std::path::PathBuf;

use crate::config;

const ENV_CONFIG: &str = "VD_FIX_LAYOUT_CONFIG";
const ENV_MODELS: &str = "VD_FIX_LAYOUT_MODELS_DIR";
const APP: &str = "vd-fix-layout";

/// Resolved config.toml path (`VD_FIX_LAYOUT_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}

/// Managed models directory (`VD_FIX_LAYOUT_MODELS_DIR` / `--download-root`).
pub fn default_models_dir() -> PathBuf {
    vd_artifact::paths::cache_dir(APP, ENV_MODELS, "models")
}

/// CLI override > env > config `download_root` > platform cache.
pub fn resolve_models_dir(cli_override: Option<PathBuf>) -> PathBuf {
    if let Some(p) = cli_override {
        return p;
    }
    if std::env::var_os(ENV_MODELS).is_some() {
        return default_models_dir();
    }
    match config::load(&config_path()) {
        Ok(cfg) => cfg
            .download_root
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(default_models_dir),
        Err(_) => default_models_dir(),
    }
}
