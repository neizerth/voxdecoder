//! Lexicon load / merge / last-wins.

use std::fs;

use tempfile::TempDir;
use vd_fix_terms::lexicon::{Lexicon, TermEntry};
use vd_fix_terms::terms::TermsLoadOptions;
use vd_fix_terms::types::Language;

#[test]
fn shipping_has_kubernetes() {
    let lex = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: true,
        terms_paths: vec![],
    })
    .unwrap();
    assert_eq!(lex.canonical_for("k8s"), Some("Kubernetes"));
    assert_eq!(lex.canonical_for("кубернетис"), Some("Kubernetes"));
}

#[test]
fn shipping_off_empty_without_terms() {
    let lex = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: false,
        terms_paths: vec![],
    })
    .unwrap();
    assert!(lex.is_empty());
}

#[test]
fn last_wins_across_terms_files() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.yaml");
    let b = dir.path().join("b.yaml");
    fs::write(&a, "canonical: Alpha\nvariants:\n  - foo\n").unwrap();
    fs::write(&b, "canonical: Beta\nvariants:\n  - foo\n").unwrap();

    let lex = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: false,
        terms_paths: vec![a, b],
    })
    .unwrap();
    assert_eq!(lex.canonical_for("foo"), Some("Beta"));
}

#[test]
fn terms_override_shipping() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("corp.yaml");
    fs::write(&path, "canonical: K8sCorp\nvariants:\n  - k8s\n").unwrap();

    let lex = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: true,
        terms_paths: vec![path],
    })
    .unwrap();
    assert_eq!(lex.canonical_for("k8s"), Some("K8sCorp"));
}

#[test]
fn yaml_multidoc() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("g.yaml");
    fs::write(
        &path,
        r"canonical: One
variants:
  - uno
---
canonical: Two
variants:
  - dos
",
    )
    .unwrap();

    let lex = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: false,
        terms_paths: vec![path],
    })
    .unwrap();
    assert_eq!(lex.canonical_for("uno"), Some("One"));
    assert_eq!(lex.canonical_for("dos"), Some("Two"));
}

#[test]
fn json_list() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("g.json");
    fs::write(&path, r#"[{"canonical":"Zap","variants":["зэп"]}]"#).unwrap();

    let lex = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: false,
        terms_paths: vec![path],
    })
    .unwrap();
    assert_eq!(lex.canonical_for("зэп"), Some("Zap"));
}

#[test]
fn markdown_arrows() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("g.md");
    fs::write(&path, "# Terms\n\n- `acme` → AcmeCloud\n- foo -> Bar\n").unwrap();

    let lex = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: false,
        terms_paths: vec![path],
    })
    .unwrap();
    assert_eq!(lex.canonical_for("acme"), Some("AcmeCloud"));
    assert_eq!(lex.canonical_for("foo"), Some("Bar"));
}

#[test]
fn missing_terms_path_exit_3() {
    let err = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: false,
        terms_paths: vec!["/no/such/glossary.yaml".into()],
    })
    .unwrap_err();
    assert_eq!(err.exit_code(), 3);
}

#[test]
fn from_entries() {
    let lex = Lexicon::from_entries(vec![TermEntry {
        canonical: "X".into(),
        variants: vec!["икс".into()],
    }]);
    assert_eq!(lex.canonical_for("икс"), Some("X"));
}
