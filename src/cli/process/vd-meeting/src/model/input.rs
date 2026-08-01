//! Input roles / sources.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRole {
    Merged,
    Participant,
    Context,
}

impl InputRole {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "merged" => Some(Self::Merged),
            "participant" => Some(Self::Participant),
            "context" => Some(Self::Context),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Merged => "merged",
            Self::Participant => "participant",
            Self::Context => "context",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSource {
    pub role: InputRole,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant: Option<String>,
}
