//! Read-only context: neighbors + `--context` materials. Never `&mut`.

mod materials;
mod neighbors;
mod visit;

pub use materials::{load_materials, Materials};
pub use neighbors::SpanContext;
pub use visit::visit_text_spans;
