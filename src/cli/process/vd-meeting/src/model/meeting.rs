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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationEnabled {
    #[default]
    Auto,
    #[serde(alias = "true")]
    True,
    #[serde(alias = "false")]
    False,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_clock_drift: Option<bool>,
}

impl Default for AlignmentOptions {
    fn default() -> Self {
        Self {
            mode: AlignmentMode::Longest,
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

#[allow(clippy::trivially_copy_pass_by_ref)] // serde skip_serializing_if signature
fn is_false(v: &bool) -> bool {
    !*v
}
