//! Neighbor window + materials handle for one span visit. Read-only.

use super::Materials;

/// Read-only hints for the current span. Never holds `&mut`.
#[derive(Debug, Clone, Copy)]
pub struct SpanContext<'a> {
    pub neighbors_before: &'a [String],
    pub neighbors_after: &'a [String],
    pub materials: &'a Materials,
}
