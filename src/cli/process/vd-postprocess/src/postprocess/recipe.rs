//! Recipe document load / validate / render.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<RecipeProviderHints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    pub outputs: Vec<RecipeOutput>,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecipeProviderHints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeOutput {
    pub id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<PathBuf>,
}

pub fn load_recipe(path: &Path) -> Result<RecipeDoc, PostprocessError> {
    let text = fs::read_to_string(path).map_err(|e| {
        if path.exists() {
            PostprocessError::Other(format!("{}: {e}", path.display()))
        } else {
            PostprocessError::NotFound(format!("recipe missing: {}", path.display()))
        }
    })?;
    let doc: RecipeDoc = if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        serde_json::from_str(&text).map_err(|e| PostprocessError::Usage(format!("recipe json: {e}")))?
    } else {
        serde_yaml::from_str(&text).map_err(|e| PostprocessError::Usage(format!("recipe yaml: {e}")))?
    };
    doc.validate(path)?;
    Ok(doc)
}

impl RecipeDoc {
    pub fn validate(&self, path: &Path) -> Result<(), PostprocessError> {
        if self.version != 1 {
            return Err(PostprocessError::Usage(format!(
                "{}: unsupported recipe version {}",
                path.display(),
                self.version
            )));
        }
        if self.outputs.is_empty() {
            return Err(PostprocessError::Usage(format!(
                "{}: recipe must declare at least one output",
                path.display()
            )));
        }
        let mut ids = std::collections::HashSet::new();
        for o in &self.outputs {
            if o.id.is_empty() || o.path.is_empty() {
                return Err(PostprocessError::Usage(format!(
                    "{}: output needs non-empty id and path",
                    path.display()
                )));
            }
            if !ids.insert(o.id.clone()) {
                return Err(PostprocessError::Usage(format!(
                    "{}: duplicate output id {}",
                    path.display(),
                    o.id
                )));
            }
        }
        if self.prompt.is_none() {
            // Body required for stub/LLM; process providers may add fields later.
            return Err(PostprocessError::Usage(format!(
                "{}: recipe needs a prompt (or future provider body)",
                path.display()
            )));
        }
        Ok(())
    }

    pub fn default_variables(&self) -> BTreeMap<String, String> {
        self.variables
            .iter()
            .map(|(k, v)| (k.clone(), v.as_default().to_string()))
            .collect()
    }
}

/// Replace `{{ key }}` with values (simple, non-nested).
pub fn render_template(template: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = template.to_string();
    for (k, v) in values {
        let patterns = [
            format!("{{{{{k}}}}}"),
            format!("{{{{ {k} }}}}"),
        ];
        for p in patterns {
            out = out.replace(&p, v);
        }
    }
    // Strip simple `{% if key %}...{% endif %}` when key empty / missing — minimal.
    out = strip_empty_if_blocks(&out, values);
    out
}

fn strip_empty_if_blocks(text: &str, values: &BTreeMap<String, String>) -> String {
    // Very small Jinja-like: {% if name %}...{% endif %}
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
        let keep = values
            .get(&name)
            .is_some_and(|v| !v.trim().is_empty());
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
