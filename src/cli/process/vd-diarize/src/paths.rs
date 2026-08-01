//! Platform paths for `vd-diarize`.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_DIARIZE_CONFIG";
const ENV_ASSETS: &str = "VD_DIARIZE_ASSETS";
const APP: &str = "vd-diarize";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}

pub fn assets_root() -> PathBuf {
    vd_artifact::paths::cache_dir(APP, ENV_ASSETS, "assets")
}
