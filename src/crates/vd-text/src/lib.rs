//! Shared local linguistic primitives (ADR 0013).
//!
//! Rust-native pieces only — terminology matching ([`term_matcher`]) and
//! text similarity ([`similarity`]). Tokenization, sentence segmentation,
//! and morphology need Natasha/razdel (Python-only today); those
//! responsibilities stay out of this crate until ADR 0013's subprocess
//! bridge is built — see the ADR's Decision for why and the design.
//!
//! No CLI, no business logic, no transcript-cleanup policy — this crate
//! only owns reusable primitives that `vd-fix-*` binaries call into.

pub mod language_packs;
pub mod linguistics;
pub mod rule_engine;
pub mod similarity;
pub mod term_matcher;
