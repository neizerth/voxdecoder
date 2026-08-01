//! Terminology rewrite behavior.

use vd_fix_terms::lexicon::{Lexicon, TermEntry};
use vd_fix_terms::terms::{TermsFixer, TermsLoadOptions};
use vd_fix_terms::types::Language;

#[test]
fn rewrites_shipping_variants() {
    let lexicon = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: true,
        terms_paths: vec![],
    })
    .unwrap();
    let fixer = TermsFixer::new(lexicon).unwrap();
    let result = fixer
        .fix("мы деплоим на кубернетис и гоняем гитхап экшенс")
        .unwrap();
    assert!(result.changed);
    assert_eq!(
        result.text,
        "мы деплоим на Kubernetes и гоняем GitHub Actions"
    );
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
    let lexicon = Lexicon::load(&TermsLoadOptions {
        language: Language::Ru,
        shipping: true,
        terms_paths: vec![],
    })
    .unwrap();
    let fixer = TermsFixer::new(lexicon).unwrap();
    let result = fixer.fix("Kubernetes already correct").unwrap();
    assert!(!result.changed);
}
