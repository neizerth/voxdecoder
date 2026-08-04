//! Meeting / SpeakerTimeline artifact shapes (Epics 4–6) — schema stubs.

use serde::{Deserialize, Serialize};

/// Speaker timeline produced by `diarize` (Epic 4).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeakerTimeline {
    pub version: u32,
    pub speakers: Vec<SpeakerSegment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlaps: Vec<OverlapRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeakerSegment {
    pub speaker: String,
    pub start_sec: f64,
    pub end_sec: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

/// Overlap / interruption region from diarize (ADR 0016).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlapRegion {
    pub start_sec: f64,
    pub end_sec: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speakers: Vec<String>,
}

/// Canonical meeting document (Epic 6) before multi-format export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeetingArtifact {
    pub version: u32,
    pub title: Option<String>,
    pub participants: Vec<String>,
    pub turns: Vec<MeetingTurn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<SpeakerTimeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeetingTurn {
    pub speaker: String,
    pub start_sec: f64,
    pub end_sec: f64,
    pub text: String,
}

/// Merge alignment strategy (Epic 5).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    Longest,
    Start,
    End,
}
