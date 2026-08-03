//! Job document types — workflow tree (`sequence` / `parallel`) + capability leaves.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// Root workflow: each entry is a node (`use: …` leaf, or `sequence` / `parallel`).
    /// A flat list of capability steps is an implicit sequence (compat).
    pub steps: Vec<WorkflowNode>,
}

impl Job {
    /// Depth-first capability leaves in declaration order.
    pub fn leaf_steps(&self) -> Vec<&Step> {
        let mut out = Vec::new();
        for n in &self.steps {
            n.collect_leaves(&mut out);
        }
        out
    }

    pub fn leaf_count(&self) -> usize {
        self.leaf_steps().len()
    }
}

/// Workflow tree node. Untagged: maps with `sequence` / `parallel` keys, else a capability [`Step`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum WorkflowNode {
    Sequence {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        sequence: Vec<WorkflowNode>,
    },
    Parallel {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        parallel: Vec<WorkflowNode>,
    },
    Step(Step),
}

impl WorkflowNode {
    pub fn step(step: Step) -> Self {
        Self::Step(step)
    }

    pub fn sequence(nodes: Vec<Self>) -> Self {
        Self::Sequence {
            id: None,
            sequence: nodes,
        }
    }

    pub fn parallel(nodes: Vec<Self>) -> Self {
        Self::Parallel {
            id: None,
            parallel: nodes,
        }
    }

    pub fn collect_leaves<'a>(&'a self, out: &mut Vec<&'a Step>) {
        match self {
            Self::Step(s) => out.push(s),
            Self::Sequence { sequence, .. } => {
                for n in sequence {
                    n.collect_leaves(out);
                }
            }
            Self::Parallel { parallel, .. } => {
                for n in parallel {
                    n.collect_leaves(out);
                }
            }
        }
    }

    pub fn is_control(&self) -> bool {
        !matches!(self, Self::Step(_))
    }
}

impl From<Step> for WorkflowNode {
    fn from(step: Step) -> Self {
        Self::Step(step)
    }
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
    /// Artifact names this step publishes (Epic 2). Empty → fall back to `id` / primary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub produces: Vec<String>,
    /// Artifact names this step requires (Epic 2). Empty → fall back to `inputs` / linear sugar.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<String>,
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
            produces: Vec::new(),
            consumes: Vec::new(),
            depends: Vec::new(),
            skip: false,
            resource: None,
            options: BTreeMap::new(),
        }
    }

    /// Effective input refs (`consumes`, else `inputs`, else sugar `input`).
    pub fn input_refs(&self) -> Vec<&str> {
        if !self.consumes.is_empty() {
            return self.consumes.iter().map(String::as_str).collect();
        }
        if !self.inputs.is_empty() {
            self.inputs.iter().map(String::as_str).collect()
        } else if let Some(i) = &self.input {
            vec![i.as_str()]
        } else {
            Vec::new()
        }
    }

    /// Names published into the artifact registry.
    pub fn produce_names(&self) -> Vec<&str> {
        if !self.produces.is_empty() {
            return self.produces.iter().map(String::as_str).collect();
        }
        let mut names = Vec::new();
        if let Some(id) = &self.id {
            names.push(id.as_str());
        }
        for k in self.outputs.keys() {
            names.push(k.as_str());
        }
        names
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
    FixDisfluency,
    FixTerms,
    FixLayout,
    FixOverlap,
    Diarize,
    MeetingMerge,
    Preprocess,
    Postprocess,
    ImportUrl,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcribe => "transcribe",
            Self::PrepareContext => "prepare-context",
            Self::FixCasing => "fix-casing",
            Self::FixAsr => "fix-asr",
            Self::FixDisfluency => "fix-disfluency",
            Self::FixTerms => "fix-terms",
            Self::FixLayout => "fix-layout",
            Self::FixOverlap => "fix-overlap",
            Self::Diarize => "diarize",
            Self::MeetingMerge => "meeting-merge",
            Self::Preprocess => "preprocess",
            Self::Postprocess => "postprocess",
            Self::ImportUrl => "import-url",
        }
    }

    pub fn is_reserved(self) -> bool {
        false
    }

    /// Default artifact kind hint for registry typing (Epic 2 / 6).
    pub fn default_artifact_kind(self) -> &'static str {
        match self {
            Self::Transcribe
            | Self::FixCasing
            | Self::FixAsr
            | Self::FixDisfluency
            | Self::FixTerms
            | Self::FixLayout
            | Self::FixOverlap => "transcript",
            Self::PrepareContext => "assets",
            Self::Diarize => "timeline",
            Self::MeetingMerge => "meeting",
            Self::Preprocess => "media",
            Self::Postprocess => "derived",
            Self::ImportUrl => "audio",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    Bool(bool),
    Number(f64),
    String(String),
    Strings(Vec<String>),
    /// Heterogeneous list (e.g. preprocess `filters:`).
    List(Vec<Self>),
    Map(BTreeMap<String, Self>),
}

impl ArgValue {
    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.clone()),
            Self::Number(n) => Some(n.to_string()),
            Self::Bool(b) => Some(b.to_string()),
            Self::Strings(_) | Self::List(_) | Self::Map(_) => None,
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

    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(v) => Some(v),
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
    /// Parse a step input/output reference.
    ///
    /// Rules:
    /// - wildcards (`prefix/*`) → [`Id`]
    /// - path separators (`/` `\`) → [`Path`]
    /// - bare names with a **known filesystem extension** (`meeting.wav`, `notes.txt`) → [`Path`]
    /// - dotted **artifact ids** (`alice.transcript`, `room.prepared`) → [`Id`]
    ///
    /// Meeting planners use `{participant}.{stage}` ids; those must not be treated as paths
    /// just because `Path::extension` is `Some`.
    pub fn parse(raw: &str) -> Self {
        if raw.ends_with("/*") || raw.contains('*') {
            return Self::Id(raw.to_string());
        }
        if is_filesystem_ref(raw) {
            Self::Path(PathBuf::from(raw))
        } else {
            Self::Id(raw.to_string())
        }
    }

    pub fn is_wildcard(&self) -> bool {
        match self {
            Self::Id(s) => s.contains('*'),
            Self::Path(_) => false,
        }
    }
}

/// True when `raw` should be resolved as a filesystem path (not an artifact id).
fn is_filesystem_ref(raw: &str) -> bool {
    if raw.contains('/') || raw.contains('\\') {
        return true;
    }
    let Some(ext) = Path::new(raw)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    else {
        return false;
    };
    // Whitelist real media/doc extensions. Do **not** treat meeting stages
    // (`.transcript`, `.cased`, `.asr`, `.text`, `.prepared`) as paths.
    matches!(
        ext.as_str(),
        "wav"
            | "mp3"
            | "m4a"
            | "ogg"
            | "flac"
            | "opus"
            | "aac"
            | "wma"
            | "mp4"
            | "mkv"
            | "mov"
            | "webm"
            | "avi"
            | "m4v"
            | "mpeg"
            | "mpg"
            | "flv"
            | "wmv"
            | "txt"
            | "md"
            | "markdown"
            | "pdf"
            | "docx"
            | "doc"
            | "rtf"
            | "csv"
            | "html"
            | "htm"
            | "srt"
            | "vtt"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "svg"
    )
}

#[derive(Debug, Clone)]
pub struct ResolvedJob {
    pub job: Job,
    pub working_dir: PathBuf,
    /// Flattened capability leaves (declaration / DFS order).
    pub steps: Vec<ResolvedStep>,
    /// Execution plan: recursive workflow over leaf indices / control structure.
    pub plan: WorkflowPlan,
    /// Legacy topo order of leaf indices (flat jobs / sequence of leaves).
    pub order: Vec<usize>,
}

/// Resolved workflow plan for the Executor (indices into [`ResolvedJob::steps`]).
#[derive(Debug, Clone)]
pub enum WorkflowPlan {
    Leaf(usize),
    Sequence(Vec<WorkflowPlan>),
    Parallel(Vec<WorkflowPlan>),
}

#[derive(Debug, Clone)]
pub struct ResolvedStep {
    pub index: u32,
    /// Dotted path in the workflow tree (e.g. `0`, `1.0`, `1.1`).
    pub path: String,
    pub capability: Capability,
    pub id: Option<String>,
    pub name: Option<String>,
    pub skip: bool,
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub outputs: BTreeMap<String, PathBuf>,
    pub produces: Vec<String>,
    pub consumes: Vec<String>,
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
