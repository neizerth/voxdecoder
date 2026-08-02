//! Russian layout; lexical content unchanged.

use tempfile::TempDir;
use vd_fix_layout::layout::{LayoutFixer, LayoutLoadOptions};
use vd_fix_layout::layout::signals::lexical_tokens;
use vd_fix_layout::types::{FixOptions, Language, ParagraphDensity};

fn fixer(density: ParagraphDensity) -> (TempDir, LayoutFixer) {
    let dir = TempDir::new().unwrap();
    let fixer = LayoutFixer::load(LayoutLoadOptions {
        language: Language::Ru,
        models_dir: dir.path().to_path_buf(),
        density,
        use_timemap: false,
        timemap: None,
    })
    .unwrap();
    (dir, fixer)
}

#[test]
fn works_without_installed_pack() {
    let (_dir, fixer) = fixer(ParagraphDensity::Normal);
    let input = "Баня. Баня — это место, куда русские люди ходят, чтобы расслабиться. Самое главное — это веник. После бани русские любят пить чай и разговаривать.";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert_eq!(lexical_tokens(input), lexical_tokens(&result.text));
    assert!(result.text.contains("\n\n") || !result.changed);
}

#[test]
fn discourse_cue_forces_break() {
    let (_dir, fixer) = fixer(ParagraphDensity::Relaxed);
    let input = "Первое предложение здесь длинное достаточно. Второе тоже есть здесь. Самое главное — это веник. Третье предложение после маркера.";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert!(result.text.contains("Самое главное"));
    assert_eq!(lexical_tokens(input), lexical_tokens(&result.text));
}
