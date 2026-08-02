//! Input resolution layer (ADR 0008): `InputSource` → `ResolvedInput`.
//!
//! Planners consume [`ResolvedInput`] artifacts. They do not consume user sources.

mod error;
mod resolve;
mod resolved;
mod source;

pub use error::InputError;
pub use resolve::{resolve, ResolveContext};
pub use resolved::{ResolvedInput, SourceKind};
pub use source::InputSource;

/// Re-export subtitle policy used by URL resolution.
pub use vd_url::SubtitlePolicy;
