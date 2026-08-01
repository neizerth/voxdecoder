//! Format-specific artifact bodies.

mod json;
mod md;
mod srt;
mod txt;
mod vtt;

pub use json::is_text_key;
pub use json::{JsonBody, JsonlBody};
pub use md::MdBody;
pub use srt::SrtBody;
pub use txt::TxtBody;
pub use vtt::{VttBlock, VttBody};
