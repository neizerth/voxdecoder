//! Meeting Model + MeetingRequest.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::InputSource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetingRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
    pub inputs: Vec<InputSource>,
    #[serde(default)]
    pub meeting: MeetingModel,
    #[serde(default)]
    pub output: MeetingOutput,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetingOutput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetingModel {
    #[serde(default)]
    pub participants: Participants,
    #[serde(default)]
    pub diarization: DiarizationPolicy,
    #[serde(default)]
    pub alignment: AlignmentOptions,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participants {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known: Vec<KnownParticipant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<CountBounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<GroupConstraints>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownParticipant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default)]
    pub constraints: ParticipantConstraints,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<AgeBounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Other,
}

impl Gender {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Male => "male",
            Self::Female => "female",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgeBounds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountBounds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub genders: BTreeMap<Gender, CountBounds>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiarizationPolicy {
    #[serde(default)]
    pub enabled: DiarizationEnabled,
}

impl Default for DiarizationPolicy {
    fn default() -> Self {
        Self {
            enabled: DiarizationEnabled::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationEnabled {
    #[default]
    Auto,
    True,
    False,
}

impl<'de> Deserialize<'de> for DiarizationEnabled {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = DiarizationEnabled;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a boolean or string (auto|true|false)")
            }

            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(if v {
                    DiarizationEnabled::True
                } else {
                    DiarizationEnabled::False
                })
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                DiarizationEnabled::parse(v).ok_or_else(|| {
                    E::unknown_variant(v, &["auto", "true", "false"])
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl DiarizationEnabled {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "true" | "yes" | "on" => Some(Self::True),
            "false" | "no" | "off" => Some(Self::False),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::True => "true",
            Self::False => "false",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignmentOptions {
    #[serde(default)]
    pub mode: AlignmentMode,
    /// What anchors final meeting timing: diarize timeline, room mix, or tracks only.
    #[serde(default)]
    pub reference: AlignmentReference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_clock_drift: Option<bool>,
}

impl Default for AlignmentOptions {
    fn default() -> Self {
        Self {
            mode: AlignmentMode::Longest,
            reference: AlignmentReference::Auto,
            tolerance_ms: None,
            allow_clock_drift: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentMode {
    #[default]
    Longest,
    Start,
    End,
}

impl AlignmentMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "longest" => Some(Self::Longest),
            "start" => Some(Self::Start),
            "end" => Some(Self::End),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Longest => "longest",
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

/// Timing / speaker-layout reference for `meeting-merge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentReference {
    /// Timeline if diarize ran; else room mix when present; else tracks only.
    #[default]
    Auto,
    /// Room mix as master clock (no diarize required).
    Mix,
    /// Require diarize `SpeakerTimeline`.
    Timeline,
    /// Ignore mix / timeline — participant transcripts only.
    None,
}

impl AlignmentReference {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "mix" | "room" => Some(Self::Mix),
            "timeline" | "diarize" => Some(Self::Timeline),
            "none" | "tracks" | "tracks_only" => Some(Self::None),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Mix => "mix",
            Self::Timeline => "timeline",
            Self::None => "none",
        }
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde skip_serializing_if signature
fn is_false(v: &bool) -> bool {
    !*v
}
