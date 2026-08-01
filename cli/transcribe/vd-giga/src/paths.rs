//! Platform paths: config file and managed model cache.

use std::env;
use std::path::PathBuf;

use directories::ProjectDirs;

const ENV_CONFIG: &str = "VD_GIGA_CONFIG";
const ENV_MODELS: &str = "VD_GIGA_MODELS_DIR";

/// Resolved config.toml path (`VD_GIGA_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    if let Ok(p) = env::var(ENV_CONFIG) {
        return PathBuf::from(p);
    }
    project_dirs()
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("vd-giga-config.toml"))
}

/// Managed checkpoint directory (`VD_GIGA_MODELS_DIR` / `--download-root`).
pub fn default_models_dir() -> PathBuf {
    if let Ok(p) = env::var(ENV_MODELS) {
        return PathBuf::from(p);
    }
    project_dirs()
        .map(|d| d.data_local_dir().join("models"))
        .unwrap_or_else(|| PathBuf::from("models"))
}

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "vd-giga")
}
