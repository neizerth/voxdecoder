//! Terminology rewrite behavior.

use vd_fix_terms::lexicon::{Lexicon, TermEntry};
use vd_fix_terms::terms::{TermsFixer, TermsLoadOptions};
use vd_fix_terms::types::Language;

#[test]
fn rewrites_terms_file_variants() {
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let terms = dir.path().join("terms.yaml");
    fs::write(
        &terms,
        "canonical: Kubernetes\nvariants:\n  - кубернетис\n  - k8s\n",
    )
    .unwrap();
    let lexicon = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: true,
        terms_paths: vec![terms],
    })
    .unwrap();
    let fixer = TermsFixer::new(lexicon).unwrap();
    let result = fixer.fix("мы деплоим на кубернетис").unwrap();
    assert!(result.changed);
    assert_eq!(result.text, "мы деплоим на Kubernetes");
}

#[test]
fn does_not_invent() {
    let lexicon = Lexicon::from_entries(vec![TermEntry {
        canonical: "Acme".into(),
        variants: vec!["акме".into()],
    }]);
    let fixer = TermsFixer::new(lexicon).unwrap();
    let result = fixer.fix("неизвестный термин остаётся").unwrap();
    assert!(!result.changed);
    assert_eq!(result.text, "неизвестный термин остаётся");
}

#[test]
fn word_boundary_no_partial() {
    let lexicon = Lexicon::from_entries(vec![TermEntry {
        canonical: "JSON".into(),
        variants: vec!["json".into()],
    }]);
    let fixer = TermsFixer::new(lexicon).unwrap();
    let result = fixer.fix("jsonl file").unwrap();
    assert!(!result.changed);
}

#[test]
fn already_canonical_unchanged() {
    let lexicon = Lexicon::from_entries(vec![TermEntry {
        canonical: "Kubernetes".into(),
        variants: vec!["кубернетис".into()],
    }]);
    let fixer = TermsFixer::new(lexicon).unwrap();
    let result = fixer.fix("Kubernetes already correct").unwrap();
    assert!(!result.changed);
}
