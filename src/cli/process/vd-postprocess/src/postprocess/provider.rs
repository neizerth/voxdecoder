//! Execution providers.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::recipe::RecipeOutput;
use super::PostprocessError;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionProviderSpec {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, serde_json::Value>,
}

impl Default for ExecutionProviderSpec {
    fn default() -> Self {
        Self {
            r#type: "stub".into(),
            model: None,
            command: None,
            options: BTreeMap::new(),
        }
    }
}

pub struct ProviderInvoke<'a> {
    pub rendered_body: &'a str,
    pub outputs: &'a [RecipeOutput],
    pub output_paths: &'a [std::path::PathBuf],
    pub variables: &'a BTreeMap<String, String>,
}

pub trait ExecutionProvider {
    fn invoke(&self, req: &ProviderInvoke<'_>) -> Result<(), PostprocessError>;
}

pub fn validate_provider_type(t: &str) -> Result<(), PostprocessError> {
    match t {
        "stub" | "openai" | "anthropic" | "ollama" | "gigachat" | "process" | "http" | "mcp" => {
            Ok(())
        }
        other => Err(PostprocessError::Usage(format!(
            "unknown provider type: {other}"
        ))),
    }
}

pub fn resolve_provider(spec: &ExecutionProviderSpec) -> Result<Box<dyn ExecutionProvider>, PostprocessError> {
    validate_provider_type(&spec.r#type)?;
    match spec.r#type.as_str() {
        "stub" => Ok(Box::new(StubProvider {
            model: spec.model.clone(),
        })),
        "openai" | "anthropic" | "ollama" | "gigachat" | "process" | "http" | "mcp" => {
            Err(PostprocessError::Unavailable(format!(
                "provider '{}' is not wired in this build; use --provider stub for local/CI",
                spec.r#type
            )))
        }
        other => Err(PostprocessError::Usage(format!(
            "unknown provider type: {other}"
        ))),
    }
}

struct StubProvider {
    model: Option<String>,
}

impl ExecutionProvider for StubProvider {
    fn invoke(&self, req: &ProviderInvoke<'_>) -> Result<(), PostprocessError> {
        for (out, path) in req.outputs.iter().zip(req.output_paths.iter()) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| PostprocessError::Other(e.to_string()))?;
            }
            let body = stub_body(out, req.rendered_body, self.model.as_deref(), req.variables);
            fs::write(path, body).map_err(|e| {
                PostprocessError::Other(format!("write {}: {e}", path.display()))
            })?;
        }
        Ok(())
    }
}

fn stub_body(
    out: &RecipeOutput,
    rendered: &str,
    model: Option<&str>,
    variables: &BTreeMap<String, String>,
) -> String {
    let fmt = out
        .format
        .as_deref()
        .or(out.mime.as_deref())
        .unwrap_or("");
    let is_json = fmt.contains("json") || Path::new(&out.path).extension().is_some_and(|e| e == "json");
    if is_json {
        let v = serde_json::json!({
            "stub": true,
            "id": out.id,
            "model": model,
            "variables": variables,
            "body": rendered,
        });
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
    } else {
        format!(
            "<!-- vd-postprocess stub provider -->\n\
             <!-- output: {} -->\n\n\
             {rendered}\n",
            out.id
        )
    }
}
