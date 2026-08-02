//! Platform paths for `vd-url`.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_URL_CONFIG";
const APP: &str = "vd-url";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}
