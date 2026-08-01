//! Platform path helpers for `vd-fix-*` binaries.

use std::env;
use std::path::PathBuf;

use directories::ProjectDirs;

/// Config file path: `$ENV` if set, else `{config_dir}/config.toml` for `app`.
pub fn config_path(app: &str, env_var: &str) -> PathBuf {
    if let Ok(p) = env::var(env_var) {
        return PathBuf::from(p);
    }
    ProjectDirs::from("", "", app)
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from(format!("{app}-config.toml")))
}

/// Cache subdirectory: `$ENV` if set, else `{cache_dir}/{subdir}` for `app`.
pub fn cache_dir(app: &str, env_var: &str, subdir: &str) -> PathBuf {
    if let Ok(p) = env::var(env_var) {
        return PathBuf::from(p);
    }
    ProjectDirs::from("", "", app)
        .map(|d| d.cache_dir().join(subdir))
        .unwrap_or_else(|| PathBuf::from(subdir))
}
