//! Input roles / sources / purposes.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Why this source is in the meeting (orthogonal to `role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputPurpose {
    /// Build a transcript branch (transcribe → fix-*).
    Transcript,
    /// Feed diarization / SpeakerTimeline (usually a room mix).
    Timeline,
}

impl InputPurpose {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "transcript" => Some(Self::Transcript),
            "timeline" => Some(Self::Timeline),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::Timeline => "timeline",
        }
    }
}

/// Provenance / kind of audio source (not the same as purpose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRole {
    /// Multi-speaker / room recording (wire: `room`; alias `merged`).
    #[serde(alias = "merged")]
    Room,
    Participant,
    Context,
}

impl InputRole {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "room" | "merged" => Some(Self::Room),
            "participant" => Some(Self::Participant),
            "context" => Some(Self::Context),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Room => "room",
            Self::Participant => "participant",
            Self::Context => "context",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSource {
    pub role: InputRole,
    /// Local media / docs path. Empty when [`Self::url`] is set.
    #[serde(default)]
    pub path: PathBuf,
    /// Online media — planner inserts `import-url`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant: Option<String>,
    /// Empty = planner fills defaults from role + sibling inputs + diarization policy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub purposes: Vec<InputPurpose>,
    /// Subtitle policy for URL imports (`ignore` | `prefer` | `require`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitles: Option<String>,
    /// Optional URL resolver hint (`stub` · `youtube` · `direct` · …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}
