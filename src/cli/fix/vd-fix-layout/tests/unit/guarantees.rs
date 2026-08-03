//! Guarantees: never changes lexical content; no split of timed unit text oddly.

use tempfile::TempDir;
use vd_fix_layout::layout::signals::lexical_tokens;
use vd_fix_layout::layout::{LayoutFixer, LayoutLoadOptions};
use vd_fix_layout::types::{FixOptions, Language, ParagraphDensity};

#[test]
fn never_changes_lexical_content() {
    let dir = TempDir::new().unwrap();
    let fixer = LayoutFixer::load(LayoutLoadOptions {
        language: Language::Ru,
        models_dir: dir.path().to_path_buf(),
        density: ParagraphDensity::Compact,
        use_timemap: false,
        timemap: None,
    })
    .unwrap();
    let input = "Мы обсуждали Kubernetes и API Gateway. Числа 42 и 3.14 остались. Имена Владимир и Core не меняются.";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert_eq!(lexical_tokens(input), lexical_tokens(&result.text));
}

#[test]
fn empty_and_short_ok() {
    let dir = TempDir::new().unwrap();
    let fixer = LayoutFixer::load(LayoutLoadOptions {
        language: Language::En,
        models_dir: dir.path().to_path_buf(),
        density: ParagraphDensity::Normal,
        use_timemap: false,
        timemap: None,
    })
    .unwrap();
    let result = fixer.fix("Hi.", FixOptions::default()).unwrap();
    assert_eq!(result.text.trim(), "Hi.");
}

#[test]
fn collapses_duplicate_periods_from_upstream() {
    let dir = TempDir::new().unwrap();
    let fixer = LayoutFixer::load(LayoutLoadOptions {
        language: Language::Ru,
        models_dir: dir.path().to_path_buf(),
        density: ParagraphDensity::Normal,
        use_timemap: false,
        timemap: None,
    })
    .unwrap();
    let input = "Баня очень похожа на сауну, но имеет свои особенности.. Каждый турист должен сходить в баню..";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert!(
        !result.text.contains(".."),
        "layout must collapse bare .. from upstream: {}",
        result.text
    );
    assert!(result.text.contains("особенности."));
    assert!(result.text.contains("баню."));
}
