//! Platform paths for `vd-pipeline`.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_PIPELINE_CONFIG";
const APP: &str = "vd-pipeline";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}
