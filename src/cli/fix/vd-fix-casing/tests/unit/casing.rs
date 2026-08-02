//! Presentation rewrite; words unchanged; `FixResult`.

use std::path::Path;

use tempfile::TempDir;
use vd_fix_casing::casing::{CasingFixer, CasingLoadOptions};
use vd_fix_casing::models;
use vd_fix_casing::types::{FixOptions, Language};

fn fixer(lang: Language) -> (TempDir, CasingFixer) {
    let dir = TempDir::new().unwrap();
    let fixer = CasingFixer::load(CasingLoadOptions {
        language: lang,
        models_dir: dir.path().to_path_buf(),
    })
    .unwrap();
    (dir, fixer)
}

#[test]
fn works_without_installed_pack() {
    let dir = TempDir::new().unwrap();
    let fixer = CasingFixer::load(CasingLoadOptions {
        language: Language::Ru,
        models_dir: dir.path().to_path_buf(),
    })
    .unwrap();
    let result = fixer
        .fix("мы обсуждали кубернетис", FixOptions::default())
        .unwrap();
    assert!(result.changed);
    assert_eq!(result.text, "Мы обсуждали кубернетис.");
}

#[test]
fn prefers_installed_pack_lexicon() {
    let dir = TempDir::new().unwrap();
    models::install(dir.path(), "ru", false, None).unwrap();
    assert!(models::is_installed(dir.path(), "ru"));
    let fixer = CasingFixer::load(CasingLoadOptions {
        language: Language::Ru,
        models_dir: dir.path().to_path_buf(),
    })
    .unwrap();
    let result = fixer
        .fix("мы обсуждали кубернетис", FixOptions::default())
        .unwrap();
    assert_eq!(result.text, "Мы обсуждали кубернетис.");
}

#[test]
fn capitalizes_and_punctuates() {
    let (_dir, fixer) = fixer(Language::Ru);
    let result = fixer
        .fix("мы обсуждали кубернетис", FixOptions::default())
        .unwrap();
    assert!(result.changed);
    assert_eq!(result.text, "Мы обсуждали кубернетис.");
}

#[test]
fn collapses_duplicate_periods() {
    let (_dir, fixer) = fixer(Language::Ru);
    let result = fixer
        .fix(
            "Первое предложение.. Второе тоже.. И третье...",
            FixOptions::default(),
        )
        .unwrap();
    assert!(!result.text.contains(".."));
    assert!(result.text.contains('…') || result.text.contains("третье."));
    assert!(result.text.contains("предложение."));
}

#[test]
fn does_not_add_period_when_present() {
    let (_dir, fixer) = fixer(Language::Ru);
    let input = "Баня очень похожа на сауну, но имеет свои особенности. Каждый турист должен сходить в баню.";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert!(!result.text.contains(".."));
    assert_eq!(
        result.text.matches('.').count(),
        input.matches('.').count()
    );
}

#[test]
fn does_not_change_words() {
    let (_dir, fixer) = fixer(Language::En);
    let result = fixer
        .fix("we discussed kubernetes", FixOptions::default())
        .unwrap();
    assert!(result.text.contains("kubernetes"));
    assert!(!result.text.contains("Kubernetes"));
}

#[test]
fn ru_quotes_and_dash() {
    let (_dir, fixer) = fixer(Language::Ru);
    let result = fixer
        .fix("он сказал \"привет\" - и ушёл", FixOptions::default())
        .unwrap();
    assert!(result.text.contains('«'));
    assert!(result.text.contains('»'));
    assert!(result.text.contains('—'));
    assert!(result.text.contains("привет"));
    assert!(result.text.contains("ушёл"));
}

#[test]
fn en_quotes_normalized() {
    let (_dir, fixer) = fixer(Language::En);
    let result = fixer.fix("he said «hello»", FixOptions::default()).unwrap();
    assert!(result.text.contains("\"hello\""));
    assert!(!result.text.contains('«'));
}

#[test]
fn capitalizes_after_sentence() {
    let (_dir, fixer) = fixer(Language::En);
    let result = fixer
        .fix("hello world. next line", FixOptions::default())
        .unwrap();
    assert_eq!(result.text, "Hello world. Next line.");
}

#[test]
fn tidy_space_before_comma() {
    let (_dir, fixer) = fixer(Language::En);
    let result = fixer.fix("hello , world", FixOptions::default()).unwrap();
    assert_eq!(result.text, "Hello, world.");
}

#[test]
fn restores_ru_asr_question() {
    let (_dir, fixer) = fixer(Language::Ru);
    let result = fixer
        .fix("как вам это нравится", FixOptions::default())
        .unwrap();
    assert!(result.text.ends_with('?'));
    assert!(result.text.starts_with('К'));
    assert!(result.text.contains("нравится"));
}

#[test]
fn restores_en_asr_question() {
    let (_dir, fixer) = fixer(Language::En);
    let result = fixer
        .fix("how does this look to you", FixOptions::default())
        .unwrap();
    assert!(result.text.ends_with('?'));
    assert!(result.text.starts_with('H'));
    assert!(result.text.contains("look"));
}

#[test]
fn restores_ru_discourse_comma() {
    let (_dir, fixer) = fixer(Language::Ru);
    let result = fixer
        .fix("ну мы пошли дальше", FixOptions::default())
        .unwrap();
    assert!(result.text.starts_with("Ну,"));
    assert!(result.text.contains("пошли"));
}

#[test]
fn restores_en_i_pronoun() {
    let (_dir, fixer) = fixer(Language::En);
    let result = fixer
        .fix("i think we should wait", FixOptions::default())
        .unwrap();
    assert!(result.text.starts_with("I "));
    assert!(result.text.contains("think"));
}

#[test]
fn long_ru_asr_gets_sentence_breaks() {
    let (_dir, fixer) = fixer(Language::Ru);
    let input = "сначала мы обсудили архитектуру потом перешли к тестам и всё проверили ещё раз";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert!(result.text.contains('.'));
    for w in [
        "сначала",
        "обсудили",
        "архитектуру",
        "перешли",
        "тестам",
        "проверили",
    ] {
        assert!(
            result.text.to_lowercase().contains(w),
            "missing word {w} in {}",
            result.text
        );
    }
}

#[test]
fn preserves_punctuation_on_already_punctuated_ru() {
    let (_dir, fixer) = fixer(Language::Ru);
    let input = "Баня. Баня — это место, куда русские люди ходят, чтобы расслабиться. \
Каждый турист должен сходить в баню.";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert!(
        result.text.contains(','),
        "commas must survive: {}",
        result.text
    );
    assert!(
        result.text.contains('—'),
        "em dash must survive: {}",
        result.text
    );
    assert!(
        result.text.matches('.').count() >= 2,
        "sentence periods must survive: {}",
        result.text
    );
    assert!(
        !result.text.contains("Баня баня это место куда"),
        "must not strip into ASR-restore mush: {}",
        result.text
    );
}

#[test]
fn install_is_idempotent() {
    let dir = TempDir::new().unwrap();
    models::install(dir.path(), "ru", false, None).unwrap();
    let again = models::install(dir.path(), "ru", false, None).unwrap();
    assert!(matches!(again, models::InstallOutcome::AlreadyPresent(_)));
    assert!(Path::new(&dir.path().join("ru/lexicon.json")).exists());
}
