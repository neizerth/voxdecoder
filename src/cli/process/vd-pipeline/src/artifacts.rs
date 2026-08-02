//! In-memory artifact registry (Epic 2): named + typed + wildcard lookup.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::job::Capability;

#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    pub name: String,
    pub kind: String,
    pub path: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct ArtifactRegistry {
    by_name: BTreeMap<String, ArtifactRecord>,
}

impl ArtifactRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, kind: impl Into<String>, path: PathBuf) {
        let name = name.into();
        self.by_name.insert(
            name.clone(),
            ArtifactRecord {
                name,
                kind: kind.into(),
                path,
            },
        );
    }

    pub fn get(&self, name: &str) -> Option<&ArtifactRecord> {
        self.by_name.get(name)
    }

    pub fn path(&self, name: &str) -> Option<&Path> {
        self.by_name.get(name).map(|r| r.path.as_path())
    }

    pub fn paths_map(&self) -> std::collections::HashMap<String, PathBuf> {
        self.by_name
            .iter()
            .map(|(k, v)| (k.clone(), v.path.clone()))
            .collect()
    }

    pub fn extend_from_map(&mut self, map: &std::collections::HashMap<String, PathBuf>, kind: &str) {
        for (k, p) in map {
            self.insert(k.clone(), kind, p.clone());
        }
    }

    /// Resolve `name` or wildcard `prefix/*` → first match (sorted by name).
    pub fn resolve(&self, pattern: &str) -> Option<&ArtifactRecord> {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            return self
                .by_name
                .iter()
                .find(|(k, _)| k.starts_with(prefix) && k.len() > prefix.len())
                .map(|(_, v)| v)
                .or_else(|| self.by_name.get(prefix));
        }
        self.by_name.get(pattern)
    }

    pub fn merge(&mut self, other: ArtifactRegistry) {
        for (k, v) in other.by_name {
            self.by_name.insert(k, v);
        }
    }

    pub fn publish_step(
        &mut self,
        capability: Capability,
        id: Option<&str>,
        produces: &[String],
        primary: &Path,
        named: &BTreeMap<String, PathBuf>,
    ) {
        let kind = capability.default_artifact_kind();
        if !produces.is_empty() {
            for name in produces {
                self.insert(name.clone(), kind, primary.to_path_buf());
            }
        }
        if let Some(id) = id {
            self.insert(id, kind, primary.to_path_buf());
        }
        for (name, path) in named {
            self.insert(name.clone(), kind, path.clone());
        }
    }
}
