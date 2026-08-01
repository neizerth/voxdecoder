//! Artifact load / text spans / write. Knows structure only — not ASR/casing/terms.

mod detect;
mod formats;
mod load;
mod text_spans;
mod writer;

pub use detect::detect_type;
pub use formats::VttBlock;
pub use load::{load, load_from_str, Artifact, ArtifactError};
pub use text_spans::{apply_to_text_spans, count_text_spans};
pub use writer::write;
