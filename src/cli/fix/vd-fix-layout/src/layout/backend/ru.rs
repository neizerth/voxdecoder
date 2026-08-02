//! Russian paragraph layout.

use crate::models::Lexicon;
use crate::types::ParagraphDensity;

use super::layout_paragraphs;

pub fn layout(text: &str, density: ParagraphDensity, lexicon: &Lexicon) -> String {
    layout_paragraphs(text, density, lexicon)
}
