//! Context materials — read-only vocabulary from `--context`.

use std::fs;

use tempfile::TempDir;
use vd_fix_asr::context::load_materials;

#[test]
fn loads_file_vocabulary() {
    let dir = TempDir::new().unwrap();
    let glossary = dir.path().join("glossary.txt");
    fs::write(&glossary, "GitHub Actions deploy staging\n").unwrap();
    let mats = load_materials(&[glossary]).unwrap();
    assert!(mats.vocabulary.contains("github"));
    assert!(mats.vocabulary.contains("actions"));
    assert!(!mats.source_paths.is_empty());
}

#[test]
fn missing_path_errors() {
    let err = load_materials(&[std::path::PathBuf::from("/no/such/context/path")]).unwrap_err();
    assert!(err.contains("missing"));
}
