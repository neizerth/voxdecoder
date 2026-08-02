//! Platform paths for `vd-preprocess`.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_PREPROCESS_CONFIG";
const APP: &str = "vd-preprocess";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}
