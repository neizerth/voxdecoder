//! ASR wording repair: recognition fixes, not canonical terms.

use std::fs;

use tempfile::TempDir;
use vd_fix_asr::asr::stages::ConfidencePolicy;
use vd_fix_asr::asr::{AsrFixer, AsrLoadOptions};
use vd_fix_asr::context::{load_materials, SpanContext};
use vd_fix_asr::types::{FixOptions, Language};

fn empty_ctx(materials: &vd_fix_asr::context::Materials) -> SpanContext<'_> {
    SpanContext {
        neighbors_before: &[],
        neighbors_after: &[],
        materials,
    }
}

#[test]
fn without_dictionary_leaves_asr_mishears() {
    let fixer = AsrFixer::load(AsrLoadOptions {
        language: Language::Ru,
        context_paths: vec![],
        neighbor_window: 1,
        confidence_policy: ConfidencePolicy::default(),
        dictionary_paths: vec![],
        project_dir: None,
    })
    .unwrap();
    let mats = fixer.materials();
    let result = fixer
        .fix_text(
            "мы используем гитхап экшенс",
            empty_ctx(mats),
            FixOptions::default(),
        )
        .unwrap();
    assert!(!result.changed);
    assert_eq!(result.text, "мы используем гитхап экшенс");
}

#[test]
fn dictionary_file_repairs_asr_mishear() {
    let dir = TempDir::new().unwrap();
    let dict = dir.path().join("asr.yml");
    fs::write(
        &dict,
        "canonical: гитхаб\nvariants:\n  - гитхап\n",
    )
    .unwrap();
    let fixer = AsrFixer::load(AsrLoadOptions {
        language: Language::Ru,
        context_paths: vec![],
        neighbor_window: 1,
        confidence_policy: ConfidencePolicy::default(),
        dictionary_paths: vec![dict],
        project_dir: None,
    })
    .unwrap();
    let mats = fixer.materials();
    let result = fixer
        .fix_text(
            "мы используем гитхап",
            empty_ctx(mats),
            FixOptions::default(),
        )
        .unwrap();
    assert!(result.changed);
    assert_eq!(result.text, "мы используем гитхаб");
}

#[test]
fn neighbor_does_not_strip_case_ending() {
    let fixer = AsrFixer::load(AsrLoadOptions {
        language: Language::Ru,
        context_paths: vec![],
        neighbor_window: 1,
        confidence_policy: ConfidencePolicy::default(),
        dictionary_paths: vec![],
        project_dir: None,
    })
    .unwrap();
    let mats = fixer.materials();
    // Neighbor "друг" must not rewrite inflected "друга".
    let before = vec!["друг".to_string()];
    let ctx = SpanContext {
        neighbors_before: &before,
        neighbors_after: &[],
        materials: mats,
    };
    let result = fixer
        .fix_text("массажировать им друг друга", ctx, FixOptions::default())
        .unwrap();
    assert!(
        result.text.contains("друга"),
        "expected inflected form kept, got {}",
        result.text
    );
}

#[test]
fn context_vocabulary_can_correct_close_token() {
    let dir = TempDir::new().unwrap();
    let ctx_file = dir.path().join("readme.md");
    fs::write(&ctx_file, "We use SafeTensors for weights.\n").unwrap();
    let fixer = AsrFixer::load(AsrLoadOptions {
        language: Language::En,
        context_paths: vec![ctx_file],
        neighbor_window: 0,
        confidence_policy: ConfidencePolicy::default(),
        dictionary_paths: vec![],
        project_dir: None,
    })
    .unwrap();
    let mats = fixer.materials();
    assert!(mats.vocabulary.contains("safetensors"));
    let result = fixer
        .fix_text("safetensores rocks", empty_ctx(mats), FixOptions::default())
        .unwrap();
    // Context vocabulary (from docs) fuzzy-corrects close ASR tokens — no in-code builtin.
    assert!(result.text.to_lowercase().contains("safetensors"));
}

#[test]
fn materials_load_is_separate_readonly() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("a.txt");
    fs::write(&f, "kubernetes cluster").unwrap();
    let mats = load_materials(&[f]).unwrap();
    assert!(mats.vocabulary.contains("kubernetes"));
}
