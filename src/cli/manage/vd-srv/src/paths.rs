//! Platform paths for `vd-srv`.

use std::path::PathBuf;

use directories::ProjectDirs;

const ENV_CONFIG: &str = "VD_SRV_CONFIG";
const ENV_DATA: &str = "VD_SRV_DATA";
const APP: &str = "vd-srv";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}

/// Durable Job / Event / log root.
pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var(ENV_DATA) {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    ProjectDirs::from("", "", APP)
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("vd-srv-data"))
}

pub fn default_socket_path(data: &std::path::Path) -> PathBuf {
    data.join("vd-srv.sock")
}

pub fn pid_path(data: &std::path::Path) -> PathBuf {
    data.join("server.pid")
}

pub fn jobs_dir(data: &std::path::Path) -> PathBuf {
    data.join("jobs")
}
