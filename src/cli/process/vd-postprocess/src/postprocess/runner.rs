//! Execution runners (`ExecutionRunner` trait + backends).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::recipe::RecipeOutput;
use super::PostprocessError;

/// How to run a graph node — not “LLM config” only.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RunnerSpec {
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, serde_json::Value>,
}

impl RunnerSpec {
    pub fn with_type(t: impl Into<String>) -> Self {
        Self {
            r#type: t.into(),
            ..Default::default()
        }
    }

    /// Merge `overlay` on top of `self` (overlay fields win when set).
    #[must_use]
    pub fn merge_overlay(&self, overlay: &Self) -> Self {
        Self {
            r#type: if overlay.r#type.is_empty() {
                self.r#type.clone()
            } else {
                overlay.r#type.clone()
            },
            model: overlay.model.clone().or_else(|| self.model.clone()),
            command: overlay.command.clone().or_else(|| self.command.clone()),
            temperature: overlay.temperature.or(self.temperature),
            url: overlay.url.clone().or_else(|| self.url.clone()),
            tool: overlay.tool.clone().or_else(|| self.tool.clone()),
            options: {
                let mut o = self.options.clone();
                o.extend(overlay.options.clone());
                o
            },
        }
    }
}

/// @deprecated use [`RunnerSpec`]
pub type ExecutionProviderSpec = RunnerSpec;

pub struct RunnerInvoke<'a> {
    pub rendered_body: &'a str,
    pub outputs: &'a [RecipeOutput],
    pub output_paths: &'a [std::path::PathBuf],
    pub variables: &'a BTreeMap<String, String>,
}

/// @deprecated use [`RunnerInvoke`]
pub type ProviderInvoke<'a> = RunnerInvoke<'a>;

pub trait ExecutionRunner {
    fn execute(&self, req: &RunnerInvoke<'_>) -> Result<(), PostprocessError>;
}

/// @deprecated use [`ExecutionRunner`]
pub trait ExecutionProvider {
    fn invoke(&self, req: &ProviderInvoke<'_>) -> Result<(), PostprocessError>;
}

impl<T: ExecutionRunner + ?Sized> ExecutionProvider for T {
    fn invoke(&self, req: &ProviderInvoke<'_>) -> Result<(), PostprocessError> {
        self.execute(req)
    }
}

pub fn validate_runner_type(t: &str) -> Result<(), PostprocessError> {
    match t {
        "stub" | "openai" | "anthropic" | "gemini" | "ollama" | "qwen" | "gigachat" | "process"
        | "python" | "bash" | "http" | "grpc" | "mcp" => Ok(()),
        other => Err(PostprocessError::Usage(format!(
            "unknown runner type: {other}"
        ))),
    }
}

/// @deprecated use [`validate_runner_type`]
pub fn validate_provider_type(t: &str) -> Result<(), PostprocessError> {
    validate_runner_type(t)
}

pub fn resolve_runner(spec: &RunnerSpec) -> Result<Box<dyn ExecutionRunner>, PostprocessError> {
    let t = if spec.r#type.is_empty() {
        "stub"
    } else {
        spec.r#type.as_str()
    };
    validate_runner_type(t)?;
    match t {
        "stub" => Ok(Box::new(StubRunner {
            model: spec.model.clone(),
        })),
        "openai" | "anthropic" | "gemini" | "ollama" | "qwen" | "gigachat" | "process"
        | "python" | "bash" | "http" | "grpc" | "mcp" => Err(PostprocessError::Unavailable(
            format!("runner '{t}' is not wired in this build; use --runner stub for local/CI"),
        )),
        other => Err(PostprocessError::Usage(format!(
            "unknown runner type: {other}"
        ))),
    }
}

/// @deprecated use [`resolve_runner`]
pub fn resolve_provider(spec: &RunnerSpec) -> Result<Box<dyn ExecutionRunner>, PostprocessError> {
    resolve_runner(spec)
}

struct StubRunner {
    model: Option<String>,
}

impl ExecutionRunner for StubRunner {
    fn execute(&self, req: &RunnerInvoke<'_>) -> Result<(), PostprocessError> {
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
        .r#type
        .as_deref()
        .or(out.mime.as_deref())
        .unwrap_or("");
    let path_hint = out.path.as_deref().unwrap_or(out.artifact.as_str());
    let is_json = fmt.contains("json")
        || Path::new(path_hint)
            .extension()
            .is_some_and(|e| e == "json");
    if is_json {
        let v = serde_json::json!({
            "stub": true,
            "id": out.artifact,
            "artifact": out.artifact,
            "model": model,
            "variables": variables,
            "body": rendered,
        });
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
    } else {
        format!(
            "<!-- vd-postprocess stub runner -->\n\
             <!-- output: {} -->\n\n\
             {rendered}\n",
            out.artifact
        )
    }
}
