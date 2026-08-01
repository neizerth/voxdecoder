//! Terms load (assets dir + text).

use std::fs;

use tempfile::TempDir;
use vd_assets::{is_assets_dir, load_dictionary, write_terms, DictionaryOptions};

#[test]
fn loads_yaml_and_writes_terms() {
    let dir = TempDir::new().unwrap();
    let g = dir.path().join("g.yaml");
    fs::write(&g, "canonical: AcmeCloud\nvariants:\n  - акмеклауд\n").unwrap();

    let dict = load_dictionary(&[g], &DictionaryOptions::default()).unwrap();
    assert!(dict.entries.iter().any(|e| e.canonical == "AcmeCloud"));

    let out = dir.path().join("assets");
    let path = write_terms(&out, &dict).unwrap();
    assert!(path.exists());
    assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("terms.yml"));

    let again = load_dictionary(&[path], &DictionaryOptions::default()).unwrap();
    assert!(again.entries.iter().any(|e| e.canonical == "AcmeCloud"));
}

#[test]
fn loads_assets_dir_as_unit() {
    let dir = TempDir::new().unwrap();
    let assets = dir.path().join("assets");
    let md = assets.join("md");
    fs::create_dir_all(&md).unwrap();
    fs::write(md.join("notes.md"), "AcmeCloud Kubernetes\n").unwrap();
    write_terms(
        &assets,
        &load_dictionary(&[md.join("notes.md")], &DictionaryOptions::default()).unwrap(),
    )
    .unwrap();

    assert!(is_assets_dir(&assets));
    let dict = load_dictionary(&[assets], &DictionaryOptions::default()).unwrap();
    assert!(dict.forms.iter().any(|f| f.contains("Kubernetes") || f.contains("AcmeCloud")));
}

#[test]
fn rejects_pdf_with_hint() {
    let dir = TempDir::new().unwrap();
    let pdf = dir.path().join("a.pdf");
    fs::write(&pdf, b"%PDF-1.4").unwrap();
    let err = load_dictionary(&[pdf], &DictionaryOptions::default()).unwrap_err();
    assert!(err.to_string().contains("vd-assets"));
}
