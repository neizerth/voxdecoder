//! Import result — platform artifacts (+ local paths for CLI text mode).

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Youtube,
    Direct,
    Stub,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Youtube => "youtube",
            Self::Direct => "direct",
            Self::Stub => "stub",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "youtube" | "yt" => Some(Self::Youtube),
            "direct" | "http" | "https" => Some(Self::Direct),
            "stub" => Some(Self::Stub),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactHandle {
    pub id: String,
    pub kind: String,
    /// Local path (CLI text mode / Executor binder). Not part of JSON machine contract.
    #[serde(skip)]
    pub path: PathBuf,
}

impl ArtifactHandle {
    pub fn new(id: impl Into<String>, kind: impl Into<String>, path: PathBuf) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub provider: ProviderId,
    pub audio: Option<ArtifactHandle>,
    pub metadata: ArtifactHandle,
    pub subtitle: Option<ArtifactHandle>,
}

impl ImportResult {
    /// Filesystem-independent JSON report entries.
    pub fn artifact_list(&self) -> Vec<ArtifactRefJson> {
        let mut out = Vec::new();
        if let Some(a) = &self.audio {
            out.push(ArtifactRefJson {
                id: a.id.clone(),
                kind: a.kind.clone(),
            });
        }
        out.push(ArtifactRefJson {
            id: self.metadata.id.clone(),
            kind: self.metadata.kind.clone(),
        });
        if let Some(s) = &self.subtitle {
            out.push(ArtifactRefJson {
                id: s.id.clone(),
                kind: s.kind.clone(),
            });
        }
        out
    }

    pub fn primary_path(&self) -> PathBuf {
        self.audio
            .as_ref()
            .map(|a| a.path.clone())
            .unwrap_or_else(|| self.metadata.path.clone())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactRefJson {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct JsonReport {
    pub ok: bool,
    pub provider: String,
    pub artifacts: Vec<ArtifactRefJson>,
}

impl ImportResult {
    pub fn json_report(&self) -> JsonReport {
        JsonReport {
            ok: true,
            provider: self.provider.as_str().to_string(),
            artifacts: self.artifact_list(),
        }
    }
}
