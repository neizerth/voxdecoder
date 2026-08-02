//! English layout; lexical content unchanged.

use tempfile::TempDir;
use vd_fix_layout::layout::signals::lexical_tokens;
use vd_fix_layout::layout::{LayoutFixer, LayoutLoadOptions};
use vd_fix_layout::types::{FixOptions, Language, ParagraphDensity};

#[test]
fn english_paragraphs_preserve_lexicon() {
    let dir = TempDir::new().unwrap();
    let fixer = LayoutFixer::load(LayoutLoadOptions {
        language: Language::En,
        models_dir: dir.path().to_path_buf(),
        density: ParagraphDensity::Relaxed,
        use_timemap: false,
        timemap: None,
    })
    .unwrap();
    let input = "First sentence about the topic. Second sentence continues. Anyway we switch focus now. Third sentence after the marker.";
    let result = fixer.fix(input, FixOptions::default()).unwrap();
    assert_eq!(lexical_tokens(input), lexical_tokens(&result.text));
}
