//! Plan and execute postprocess requests.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::provider::{self, ExecutionProviderSpec};
use super::recipe::{self, RecipeDoc};
use super::result::{DerivedArtifact, PostprocessResult, RecipeResult};
use super::PostprocessError;

/// Named artifact binding: path on disk (ids resolved by caller / CLI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactBinding {
    pub path: PathBuf,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostprocessRequest {
    pub inputs: BTreeMap<String, ArtifactBinding>,
    pub recipes: Vec<PathBuf>,
    pub provider: ExecutionProviderSpec,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedRecipe {
    pub recipe_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_id: Option<String>,
    pub rendered_body: String,
    pub outputs: Vec<PlannedOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedOutput {
    pub id: String,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub provider: ExecutionProviderSpec,
    pub steps: Vec<PlannedRecipe>,
}

/// Build an [`ExecutionPlan`] without invoking the provider.
pub fn plan(req: &PostprocessRequest) -> Result<ExecutionPlan, PostprocessError> {
    if req.recipes.is_empty() {
        return Err(PostprocessError::Usage("no recipes specified".into()));
    }
    if req.inputs.is_empty() {
        return Err(PostprocessError::Usage("no inputs specified".into()));
    }

    // Resolve provider early so dry-run shows the same backend a run would use.
    let mut provider = req.provider.clone();
    if provider.r#type.is_empty() {
        provider.r#type = "stub".into();
    }
    provider::validate_provider_type(&provider.r#type)?;

    let output_dir = req
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    let mut steps = Vec::with_capacity(req.recipes.len());
    for recipe_path in &req.recipes {
        let doc = recipe::load_recipe(recipe_path)?;
        validate_inputs(&doc, &req.inputs, recipe_path)?;
        let mut vars = doc.default_variables();
        for (k, v) in &req.variables {
            vars.insert(k.clone(), v.clone());
        }
        for (name, binding) in &req.inputs {
            let text = read_input(&binding.path)?;
            vars.insert(name.clone(), text);
        }
        if let Some(hints) = &doc.provider {
            if let Some(t) = &hints.temperature {
                provider
                    .options
                    .entry("temperature".into())
                    .or_insert(serde_json::json!(t));
            }
            if provider.model.is_none() {
                if let Some(m) = &hints.model {
                    provider.model = Some(m.clone());
                }
            }
        }

        let prompt = doc.prompt.as_deref().unwrap_or("");
        let rendered_body = recipe::render_template(prompt, &vars);

        let mut outputs = Vec::new();
        for o in &doc.outputs {
            let path = recipe::resolve_output_path(&o.path, &output_dir, &vars);
            outputs.push(PlannedOutput {
                id: o.id.clone(),
                path,
                mime: o.mime.clone(),
                format: o.format.clone(),
            });
        }

        steps.push(PlannedRecipe {
            recipe_path: recipe_path.clone(),
            recipe_id: doc.id.clone(),
            rendered_body,
            outputs,
        });
    }

    Ok(ExecutionPlan { provider, steps })
}

/// Plan then execute; returns registered derived artifacts.
pub fn execute(req: &PostprocessRequest) -> Result<PostprocessResult, PostprocessError> {
    let plan = plan(req)?;
    let backend = provider::resolve_provider(&plan.provider)?;

    let mut results = Vec::new();
    for step in &plan.steps {
        let doc = recipe::load_recipe(&step.recipe_path)?;
        let paths: Vec<PathBuf> = step.outputs.iter().map(|o| o.path.clone()).collect();
        for path in &paths {
            if path.exists() && !req.overwrite {
                return Err(PostprocessError::Usage(format!(
                    "output exists (pass --overwrite): {}",
                    path.display()
                )));
            }
        }
        let vars = {
            let mut v = doc.default_variables();
            v.extend(req.variables.clone());
            for (name, binding) in &req.inputs {
                v.insert(name.clone(), read_input(&binding.path)?);
            }
            v
        };
        backend.invoke(&provider::ProviderInvoke {
            rendered_body: &step.rendered_body,
            outputs: &doc.outputs,
            output_paths: &paths,
            variables: &vars,
        })?;

        results.push(RecipeResult {
            recipe_id: step.recipe_id.clone(),
            outputs: step
                .outputs
                .iter()
                .map(|o| DerivedArtifact {
                    id: o.id.clone(),
                    path: o.path.clone(),
                })
                .collect(),
        });
    }
    Ok(PostprocessResult { results })
}

fn validate_inputs(
    doc: &RecipeDoc,
    inputs: &BTreeMap<String, ArtifactBinding>,
    recipe_path: &Path,
) -> Result<(), PostprocessError> {
    for (name, decl) in &doc.inputs {
        if decl.required && !inputs.contains_key(name) {
            return Err(PostprocessError::Usage(format!(
                "{}: missing required input '{name}'",
                recipe_path.display()
            )));
        }
    }
    for (name, binding) in inputs {
        if !binding.path.exists() {
            return Err(PostprocessError::NotFound(format!(
                "input '{name}' missing: {}",
                binding.path.display()
            )));
        }
    }
    Ok(())
}

fn read_input(path: &Path) -> Result<String, PostprocessError> {
    if path.is_dir() {
        return Ok(format!("(directory {})", path.display()));
    }
    fs::read_to_string(path).map_err(|e| {
        PostprocessError::NotFound(format!("{}: {e}", path.display()))
    })
}
