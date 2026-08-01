//! Job document types.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub input: JobInput,
    #[serde(default)]
    pub context: JobContext,
    #[serde(default)]
    pub output: JobOutput,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_parallel: Option<u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resources: BTreeMap<String, u32>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobOutput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Step {
    #[serde(rename = "use")]
    pub r#use: Capability,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Sugar for a single-entry [`Self::inputs`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub outputs: BTreeMap<String, PathBuf>,
    /// Ordering edges to other step `id`s (no data required).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub skip: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, ArgValue>,
}

impl Step {
    pub fn new(r#use: Capability) -> Self {
        Self {
            r#use,
            id: None,
            name: None,
            input: None,
            inputs: Vec::new(),
            output: None,
            outputs: BTreeMap::new(),
            depends: Vec::new(),
            skip: false,
            resource: None,
            options: BTreeMap::new(),
        }
    }

    /// Effective input refs (`inputs`, or sugar `input`).
    pub fn input_refs(&self) -> Vec<&str> {
        if !self.inputs.is_empty() {
            self.inputs.iter().map(String::as_str).collect()
        } else if let Some(i) = &self.input {
            vec![i.as_str()]
        } else {
            Vec::new()
        }
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde skip_serializing_if signature
fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Transcribe,
    PrepareContext,
    FixCasing,
    FixAsr,
    FixTerms,
    Diarize,
    MeetingMerge,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcribe => "transcribe",
            Self::PrepareContext => "prepare-context",
            Self::FixCasing => "fix-casing",
            Self::FixAsr => "fix-asr",
            Self::FixTerms => "fix-terms",
            Self::Diarize => "diarize",
            Self::MeetingMerge => "meeting-merge",
        }
    }

    pub fn is_reserved(self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    Bool(bool),
    Number(f64),
    String(String),
    Strings(Vec<String>),
    Map(BTreeMap<String, Self>),
}

impl ArgValue {
    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.clone()),
            Self::Number(n) => Some(n.to_string()),
            Self::Bool(b) => Some(b.to_string()),
            Self::Strings(_) | Self::Map(_) => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscribeEngine {
    Gigaam,
    Whisper,
}

impl TranscribeEngine {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "gigaam" => Some(Self::Gigaam),
            "whisper" => Some(Self::Whisper),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gigaam => "gigaam",
            Self::Whisper => "whisper",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactRef {
    Id(String),
    Path(PathBuf),
}

impl ArtifactRef {
    pub fn parse(raw: &str) -> Self {
        let p = PathBuf::from(raw);
        if raw.contains('/') || raw.contains('\\') || p.extension().is_some() {
            Self::Path(p)
        } else {
            Self::Id(raw.to_string())
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedJob {
    pub job: Job,
    pub working_dir: PathBuf,
    pub steps: Vec<ResolvedStep>,
    /// Topological execution order (indices into `steps` / `job.steps`).
    pub order: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct ResolvedStep {
    pub index: u32,
    pub capability: Capability,
    pub id: Option<String>,
    pub name: Option<String>,
    pub skip: bool,
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub outputs: BTreeMap<String, PathBuf>,
    pub options: BTreeMap<String, ArgValue>,
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Reserved(String),
    #[error("{0}")]
    Other(String),
}

impl JobError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::Reserved(_) => 2,
            Self::NotFound(_) => 3,
            Self::Other(_) => 1,
        }
    }
}
