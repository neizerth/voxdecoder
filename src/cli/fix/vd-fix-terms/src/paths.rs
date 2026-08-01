//! Platform paths: config file and reserved asset cache.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_FIX_TERMS_CONFIG";
const ENV_CACHE: &str = "VD_FIX_TERMS_MODELS_DIR";
const APP: &str = "vd-fix-terms";

/// Resolved config.toml path (`VD_FIX_TERMS_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}

/// Reserved download / assets directory (platform cache). Unused until packs ship.
pub fn default_assets_dir() -> PathBuf {
    vd_artifact::paths::cache_dir(APP, ENV_CACHE, "models")
}
