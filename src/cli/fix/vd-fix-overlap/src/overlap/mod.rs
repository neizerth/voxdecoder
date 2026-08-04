//! Duplicated-speech detection (ADR 0012 §2).
//!
//! Pure, deterministic, no I/O — see `detect.rs` for the algorithm and
//! `STRUCTURE.md` for why this crate stops at detection instead of also
//! fixing (removing/trimming) artifacts.

mod detect;

pub use detect::{
    detect_duplicates, normalize_for_compare, DetectOptions, DuplicateKind, DuplicatePair,
    TimelineHint, TrimAction, Utterance,
};
