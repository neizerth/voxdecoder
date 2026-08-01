//! Platform GigaAM cache / models dir.

use std::path::PathBuf;

use vd_gigaam::paths;

#[test]
fn preferred_respects_xdg_cache_home() {
    let prev = std::env::var_os("XDG_CACHE_HOME");
    // SAFETY: test process isolation; restore after.
    std::env::set_var("XDG_CACHE_HOME", "/tmp/xdg-cache-test");
    let got = paths::preferred_gigaam_cache();
    assert_eq!(got, PathBuf::from("/tmp/xdg-cache-test/gigaam"));
    match prev {
        Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
        None => std::env::remove_var("XDG_CACHE_HOME"),
    }
}

#[test]
fn default_models_dir_env_override() {
    let prev = std::env::var_os("VD_GIGAAM_MODELS_DIR");
    std::env::set_var("VD_GIGAAM_MODELS_DIR", "/tmp/vd-gigaam-models-test");
    assert_eq!(
        paths::default_models_dir(),
        PathBuf::from("/tmp/vd-gigaam-models-test")
    );
    match prev {
        Some(v) => std::env::set_var("VD_GIGAAM_MODELS_DIR", v),
        None => std::env::remove_var("VD_GIGAAM_MODELS_DIR"),
    }
}

#[test]
fn candidates_include_preferred() {
    let preferred = paths::preferred_gigaam_cache();
    let candidates = paths::gigaam_cache_candidates();
    assert!(
        candidates.contains(&preferred),
        "preferred {preferred:?} not in {candidates:?}"
    );
}
