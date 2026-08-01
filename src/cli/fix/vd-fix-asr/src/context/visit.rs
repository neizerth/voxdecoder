//! Visit spans with ASR read-only neighbor + materials context.

use vd_artifact::{apply_to_text_spans, collect_span_texts, Artifact, TextSpan};

use super::{Materials, SpanContext};

/// Visit each mutable transcript text span with read-only neighbor + materials context.
pub fn visit_text_spans<E, F>(
    artifact: &mut Artifact,
    neighbor_window: u32,
    materials: &Materials,
    mut f: F,
) -> Result<(), E>
where
    F: FnMut(TextSpan<'_>, SpanContext<'_>) -> Result<(), E>,
{
    let snapshot = collect_span_texts(artifact);
    let window = neighbor_window as usize;
    let mut index = 0usize;
    apply_to_text_spans(artifact, |span| {
        let before_start = index.saturating_sub(window);
        let neighbors_before = &snapshot[before_start..index];
        let after_end = (index + 1 + window).min(snapshot.len());
        let neighbors_after = if index + 1 < snapshot.len() {
            &snapshot[index + 1..after_end]
        } else {
            &[]
        };
        let ctx = SpanContext {
            neighbors_before,
            neighbors_after,
            materials,
        };
        let result = f(span, ctx);
        index += 1;
        result
    })
}
