//! Platform paths: config file and reserved asset cache.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_FIX_ASR_CONFIG";
const ENV_MODELS: &str = "VD_FIX_ASR_MODELS_DIR";
const APP: &str = "vd-fix-asr";

/// Resolved config.toml path (`VD_FIX_ASR_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}

/// Reserved download / assets directory (platform cache). Not required for builtin backend.
pub fn default_assets_dir() -> PathBuf {
    vd_artifact::paths::cache_dir(APP, ENV_MODELS, "models")
}
