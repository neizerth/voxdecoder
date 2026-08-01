//! Platform paths for `vd-postprocess`.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_POSTPROCESS_CONFIG";
const APP: &str = "vd-postprocess";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}
