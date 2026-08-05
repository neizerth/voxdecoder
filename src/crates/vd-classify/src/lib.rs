//! Meeting-input filename classification heuristics (ADR 0017 Decision H).
//!
//! Single source of truth for the rules `skills/vd-meeting/skill.md` currently documents as
//! prose (**Filename heuristics** / **Gender** / **Mix + tracks** sections) — pure functions,
//! no I/O, unit-tested directly. Consumers:
//!
//! - `vd-meeting --interactive` (ADR 0017 Decision D) calls this crate in-process.
//! - `vd-srv`'s `plan.classify` Runtime API method calls it; `vd-mcp` gateways it as the
//!   `classify_meeting_inputs` MCP tool (Decision H) — the Skill calls the tool instead of
//!   re-deriving the rules from prose.
//!
//! Skeleton status: signatures and doc-comments here are the contract; [`strip_basename_noise`]
//! has a first working implementation as the pattern to follow. [`is_mix_token`],
//! [`infer_gender`], and [`classify_inputs`] are stubs (`todo!()`) — filling them in against the
//! skill.md tables (transcribed into doc-comments below) is tracked separately.

mod classify;
mod gender;
mod mix;
mod strip;

pub use classify::classify_inputs;
pub use gender::{infer_gender, Gender};
pub use mix::is_mix_token;
pub use strip::strip_basename_noise;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Proposed role for one input file, before user confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Shared room / mix recording (`skill.md` alias: `merged`).
    Room,
    /// Per-speaker track.
    Participant,
}

/// One file's proposed classification — what `vd-meeting --interactive` (Decision D) shows
/// the user before they accept/edit/drop it, and what `plan.classify` / `classify_meeting_inputs`
/// (ADR 0017 Decision H) return over the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifiedInput {
    pub path: PathBuf,
    pub role: Role,
    /// Cleaned candidate name — `participant` id when `role == Participant`; the original
    /// script/casing is preserved (skill.md: never transliterate Cyrillic → Latin here).
    pub name: String,
    /// `None` when not confidently inferable — callers must not guess further, only ask.
    pub gender: Option<Gender>,
}
