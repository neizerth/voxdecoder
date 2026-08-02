//! Recipe document load / validate / render.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

use super::runner::RunnerSpec;
use super::PostprocessError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDoc {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, RecipeInputDecl>,
    #[serde(default)]
    pub variables: BTreeMap<String, RecipeVarValue>,
    /// Secret refs only (`env:NAME`). Never plain values in packs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,
    /// Default runner for graph nodes that omit `runner` (`provider` alias).
    #[serde(
        default,
        alias = "provider",
        skip_serializing_if = "Option::is_none"
    )]
    pub runner: Option<RunnerSpec>,
    /// Top-level prompt sugar → single `graph` node `main` when `graph` empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, deserialize_with = "deserialize_outputs")]
    pub outputs: Vec<RecipeOutput>,
    #[serde(default)]
    pub graph: Vec<GraphNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeInputDecl {
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

impl Default for RecipeInputDecl {
    fn default() -> Self {
        Self { required: true }
    }
}

/// Variable default in recipe: bare string or `{ default: "…" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecipeVarValue {
    String(String),
    Object { default: String },
}

impl RecipeVarValue {
    pub fn as_default(&self) -> &str {
        match self {
            Self::String(s) => s,
            Self::Object { default } => default,
        }
    }
}

/// Declared derived artifact (`artifact` id + `type` + optional `path`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeOutput {
    pub artifact: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(
        default,
        rename = "type",
        alias = "format",
        skip_serializing_if = "Option::is_none"
    )]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<PathBuf>,
}

impl RecipeOutput {
    /// Legacy accessor — same as [`Self::artifact`].
    pub fn id(&self) -> &str {
        &self.artifact
    }

    pub fn resolved_path_pattern(&self) -> String {
        if let Some(p) = &self.path {
            return p.clone();
        }
        default_path_for(&self.artifact, self.r#type.as_deref())
    }
}

pub fn default_path_for(artifact: &str, typ: Option<&str>) -> String {
    let ext = match typ.map(str::to_ascii_lowercase).as_deref() {
        Some("markdown" | "md" | "text/markdown") => "md",
        Some("json" | "application/json") => "json",
        Some("csv" | "text/csv") => "csv",
        Some("yaml" | "yml") => "yaml",
        _ => "txt",
    };
    format!("{artifact}.{ext}")
}

/// Unified input: external artifact or upstream node output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputRef {
    From {
        from: String,
    },
    Artifact {
        artifact: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<RunnerSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, InputRef>,
    #[serde(default, deserialize_with = "deserialize_outputs_opt")]
    pub outputs: Vec<RecipeOutput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Reserved — expand one node into N planned nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreach: Option<serde_yaml::Value>,
}

fn deserialize_outputs<'de, D>(deserializer: D) -> Result<Vec<RecipeOutput>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_yaml::Value::deserialize(deserializer)?;
    parse_outputs_value(&value).map_err(D::Error::custom)
}

fn deserialize_outputs_opt<'de, D>(deserializer: D) -> Result<Vec<RecipeOutput>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_yaml::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_yaml::Value::Null) => Ok(Vec::new()),
        Some(v) => parse_outputs_value(&v).map_err(D::Error::custom),
    }
}

fn parse_outputs_value(value: &serde_yaml::Value) -> Result<Vec<RecipeOutput>, String> {
    match value {
        serde_yaml::Value::Sequence(seq) => {
            let mut out = Vec::with_capacity(seq.len());
            for item in seq {
                let map = item
                    .as_mapping()
                    .ok_or_else(|| "output list item must be a mapping".to_string())?;
                let artifact = map_str(map, "artifact")
                    .or_else(|| map_str(map, "id"))
                    .ok_or_else(|| "output needs artifact or id".to_string())?;
                let path = map_str(map, "path");
                let r#type = map_str(map, "type").or_else(|| map_str(map, "format"));
                let mime = map_str(map, "mime");
                let schema = map_str(map, "schema").map(PathBuf::from);
                if path.is_none() && r#type.is_none() {
                    // legacy list often has path; allow type-only / path-only
                }
                out.push(RecipeOutput {
                    artifact,
                    path,
                    r#type,
                    mime,
                    schema,
                });
            }
            Ok(out)
        }
        serde_yaml::Value::Mapping(map) => {
            let mut out = Vec::with_capacity(map.len());
            for (k, v) in map {
                let key = k
                    .as_str()
                    .ok_or_else(|| "output map key must be a string".to_string())?
                    .to_string();
                let inner = v.as_mapping();
                let artifact = inner
                    .and_then(|m| map_str(m, "artifact"))
                    .unwrap_or_else(|| key.clone());
                let path = inner.and_then(|m| map_str(m, "path"));
                let r#type = inner
                    .and_then(|m| map_str(m, "type").or_else(|| map_str(m, "format")));
                let mime = inner.and_then(|m| map_str(m, "mime"));
                let schema = inner
                    .and_then(|m| map_str(m, "schema"))
                    .map(PathBuf::from);
                out.push(RecipeOutput {
                    artifact,
                    path,
                    r#type,
                    mime,
                    schema,
                });
            }
            Ok(out)
        }
        _ => Err("outputs must be a list or a map".into()),
    }
}

fn map_str(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    map.get(serde_yaml::Value::String(key.into()))
        .and_then(|v| v.as_str().map(str::to_string))
}

pub fn load_recipe(path: &Path) -> Result<RecipeDoc, PostprocessError> {
    let text = fs::read_to_string(path).map_err(|e| {
        if path.exists() {
            PostprocessError::Other(format!("{}: {e}", path.display()))
        } else {
            PostprocessError::NotFound(format!("recipe missing: {}", path.display()))
        }
    })?;
    let mut doc: RecipeDoc = if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        // JSON via yaml Value bridge for shared outputs deserializer path:
        let v: serde_yaml::Value = serde_json::from_str(&text)
            .map_err(|e| PostprocessError::Usage(format!("recipe json: {e}")))?;
        serde_yaml::from_value(v)
            .map_err(|e| PostprocessError::Usage(format!("recipe json: {e}")))?
    } else {
        serde_yaml::from_str(&text)
            .map_err(|e| PostprocessError::Usage(format!("recipe yaml: {e}")))?
    };
    doc.normalize();
    doc.validate(path)?;
    Ok(doc)
}

impl RecipeDoc {
    /// Expand top-level `prompt`/`command` into a single graph node when `graph` is empty.
    pub fn normalize(&mut self) {
        if self.graph.is_empty() {
            if self.prompt.is_some() || self.command.is_some() {
                self.graph.push(GraphNode {
                    id: "main".into(),
                    runner: None,
                    needs: Vec::new(),
                    inputs: BTreeMap::new(),
                    outputs: self.outputs.clone(),
                    prompt: self.prompt.clone(),
                    command: self.command.clone(),
                    foreach: None,
                });
            }
        } else {
            // Fill empty node outputs from recipe-level outputs for single-node graphs.
            if self.graph.len() == 1 && self.graph[0].outputs.is_empty() && !self.outputs.is_empty()
            {
                self.graph[0].outputs = self.outputs.clone();
            }
        }
    }

    pub fn validate(&self, path: &Path) -> Result<(), PostprocessError> {
        if self.version != 1 {
            return Err(PostprocessError::Usage(format!(
                "{}: unsupported recipe version {}",
                path.display(),
                self.version
            )));
        }
        if self.graph.is_empty() {
            return Err(PostprocessError::Usage(format!(
                "{}: recipe needs a graph (or top-level prompt)",
                path.display()
            )));
        }
        let recipe_outputs = if self.outputs.is_empty() {
            // Allow outputs only on nodes.
            self.graph.iter().flat_map(|n| n.outputs.iter()).count() > 0
        } else {
            true
        };
        if !recipe_outputs && self.graph.iter().all(|n| n.outputs.is_empty()) {
            return Err(PostprocessError::Usage(format!(
                "{}: recipe must declare at least one output",
                path.display()
            )));
        }

        let mut node_ids = std::collections::HashSet::new();
        for node in &self.graph {
            if node.id.is_empty() {
                return Err(PostprocessError::Usage(format!(
                    "{}: graph node needs non-empty id",
                    path.display()
                )));
            }
            if !node_ids.insert(node.id.clone()) {
                return Err(PostprocessError::Usage(format!(
                    "{}: duplicate graph node id {}",
                    path.display(),
                    node.id
                )));
            }
            if node.foreach.is_some() {
                return Err(PostprocessError::Usage(format!(
                    "{}: foreach is reserved (not implemented yet) on node '{}'",
                    path.display(),
                    node.id
                )));
            }
            if node.prompt.is_none() && node.command.is_none() {
                // Allow empty body for future http/mcp; stub needs prompt or command.
                // Soft: ok for now if runner is process with command on runner spec.
            }
            for o in &node.outputs {
                validate_output(path, o)?;
            }
        }
        for o in &self.outputs {
            validate_output(path, o)?;
        }
        for (name, refer) in &self.secrets {
            if !refer.starts_with("env:") && !refer.starts_with("vault:") && !refer.starts_with("file:")
            {
                return Err(PostprocessError::Usage(format!(
                    "{}: secret '{name}' must be a ref (env:NAME), not a plain value",
                    path.display()
                )));
            }
        }
        // needs must reference known ids
        for node in &self.graph {
            for dep in &node.needs {
                if !node_ids.contains(dep) {
                    return Err(PostprocessError::Usage(format!(
                        "{}: node '{}': unknown needs '{dep}'",
                        path.display(),
                        node.id
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn default_variables(&self) -> BTreeMap<String, String> {
        self.variables
            .iter()
            .map(|(k, v)| (k.clone(), v.as_default().to_string()))
            .collect()
    }

    /// Resolve `secrets` refs into values (not for dry-run display).
    pub fn resolve_secrets(&self) -> Result<BTreeMap<String, String>, PostprocessError> {
        let mut out = BTreeMap::new();
        for (name, refer) in &self.secrets {
            if let Some(env_key) = refer.strip_prefix("env:") {
                let val = std::env::var(env_key).map_err(|_| {
                    PostprocessError::Usage(format!(
                        "secret '{name}': env var '{env_key}' not set"
                    ))
                })?;
                out.insert(name.clone(), val);
            } else {
                return Err(PostprocessError::Usage(format!(
                    "secret '{name}': unsupported ref '{refer}'"
                )));
            }
        }
        Ok(out)
    }
}

fn validate_output(path: &Path, o: &RecipeOutput) -> Result<(), PostprocessError> {
    if o.artifact.is_empty() {
        return Err(PostprocessError::Usage(format!(
            "{}: output needs non-empty artifact id",
            path.display()
        )));
    }
    Ok(())
}

/// Replace `{{ key }}` with values (simple, non-nested).
pub fn render_template(template: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = template.to_string();
    for (k, v) in values {
        let patterns = [format!("{{{{{k}}}}}"), format!("{{{{ {k} }}}}")];
        for p in patterns {
            out = out.replace(&p, v);
        }
    }
    out = strip_empty_if_blocks(&out, values);
    out
}

fn strip_empty_if_blocks(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find("{% if ") {
        let after = &result[start + 6..];
        let Some(end_name) = after.find("%}") else {
            break;
        };
        let name = after[..end_name].trim().to_string();
        let block_start = start;
        let content_start = start + 6 + end_name + 2;
        let Some(rel_end) = result[content_start..].find("{% endif %}") else {
            break;
        };
        let content_end = content_start + rel_end;
        let block_end = content_end + "{% endif %}".len();
        let keep = values.get(&name).is_some_and(|v| !v.trim().is_empty());
        let replacement = if keep {
            result[content_start..content_end].to_string()
        } else {
            String::new()
        };
        result.replace_range(block_start..block_end, &replacement);
    }
    result
}

pub fn resolve_output_path(
    pattern: &str,
    output_dir: &Path,
    variables: &BTreeMap<String, String>,
) -> PathBuf {
    let rendered = render_template(pattern, variables);
    let p = PathBuf::from(&rendered);
    if p.is_absolute() {
        p
    } else {
        output_dir.join(p)
    }
}
