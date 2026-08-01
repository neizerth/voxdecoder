//! Config get/set.

use tempfile::TempDir;
use vd_diarize::config::{self, FileConfig};

#[test]
fn save_load_nested_backend() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    let mut cfg = FileConfig::default();
    cfg.set("backend.provider", "stub").unwrap();
    cfg.set("backend.model", "deterministic-v1").unwrap();
    cfg.set("progress", "json").unwrap();
    config::save(&path, &cfg).unwrap();
    let loaded = config::load(&path).unwrap();
    assert_eq!(loaded.provider.as_deref(), Some("stub"));
    assert_eq!(loaded.model.as_deref(), Some("deterministic-v1"));
    assert_eq!(loaded.progress.as_deref(), Some("json"));
}
