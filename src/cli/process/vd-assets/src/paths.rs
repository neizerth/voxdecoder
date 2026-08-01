//! Platform paths for `vd-assets`.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_ASSETS_CONFIG";
const ENV_CACHE: &str = "VD_ASSETS_CACHE";
const APP: &str = "vd-assets";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}

/// Extract / convert cache root (`$VD_ASSETS_CACHE` or platform cache).
pub fn cache_root() -> PathBuf {
    vd_artifact::paths::cache_dir(APP, ENV_CACHE, "extract")
}
