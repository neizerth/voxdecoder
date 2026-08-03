//! Platform paths: config file location.
//!
//! Only pulls in `vd_artifact::paths::config_path` — not the artifact /
//! `TextSpan` machinery (see `STRUCTURE.md`).

use std::path::PathBuf;

const ENV_CONFIG: &str = "VD_FIX_OVERLAP_CONFIG";
const APP: &str = "vd-fix-overlap";

/// Resolved config.toml path (`VD_FIX_OVERLAP_CONFIG` overrides).
pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}
