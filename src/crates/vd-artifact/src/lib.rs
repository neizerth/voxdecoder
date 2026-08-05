//! Transcript artifact I/O: detect, load, `TextSpan` walk, write.
//!
//! Shared by `vd-fix-*` (and any CLI that edits the same artifact shapes).
//! Does **not** own presentation / ASR / terms backends, output-path policy, or progress UX.
//!
//! Also owns [`TimeMap`] (processed → original timeline) used by preprocess + the Job Executor
//! — see `docs/adr/0001-platform-refactoring-plan.md`.

mod detect;
mod formats;
mod load;
mod segments;
mod text_spans;
mod timeline;
mod timemap;
mod writer;

pub mod paths;
pub mod types;

pub use detect::detect_type;
pub use formats::VttBlock;
pub use load::{load, load_from_str, Artifact, ArtifactError};
pub use segments::{collect_segments, remove_segments, set_segment_text, Segment, SegmentId};
pub use text_spans::{apply_to_text_spans, collect_span_texts, count_text_spans};
pub use timeline::{remap_segments_json, remap_segments_value, remap_srt_file, remap_srt_text};
pub use timemap::{TimeInterval, TimeMap, TimeMapSegment};
pub use paths::{
    atomic_temp_path, content_hash_key, finalize_atomic, job_cache_dir, job_cache_root,
    new_job_id,
};
pub use types::{ArtifactType, FixOptions, FixResult, Language, SpanId, TextSpan};
pub use writer::write;
