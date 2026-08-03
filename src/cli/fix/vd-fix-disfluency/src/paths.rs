//! Platform paths: config file location.

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_FIX_DISFLUENCY_CONFIG";
const APP: &str = "vd-fix-disfluency";

/// Resolved config.toml path (`VD_FIX_DISFLUENCY_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}
