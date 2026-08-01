//! Platform paths for `vd-meeting`.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_MEETING_CONFIG";
const APP: &str = "vd-meeting";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}
