//! Plan and execute postprocess requests.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::recipe::{self, GraphNode, InputRef, RecipeDoc, RecipeOutput};
use super::result::{DerivedArtifact, PostprocessResult, RecipeResult};
use super::runner::{self, RunnerSpec};
use super::PostprocessError;

/// Named input binding: resolved filesystem path (+ optional artifact id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactBinding {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// @deprecated alias — prefer [`ArtifactBinding`].
pub type InputBinding = ArtifactBinding;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostprocessRequest {
    pub inputs: BTreeMap<String, ArtifactBinding>,
    pub recipes: Vec<PathBuf>,
    /// Override after Config; applied as `CLI > Job > Config > Recipe`.
    /// Serde accepts legacy `provider` key.
    #[serde(default, alias = "provider")]
    pub runner: RunnerSpec,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactOutput {
    pub artifact: String,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionNode {
    pub id: String,
    /// Original graph node id (without recipe prefix).
    pub graph_node_id: String,
    pub recipe_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_id: Option<String>,
    pub runner: RunnerSpec,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
    pub rendered_body: String,
    pub outputs: Vec<ArtifactOutput>,
    /// Parallel-capable when `needs` is empty (documented contract).
    #[serde(default)]
    pub parallel: bool,
}

/// First-class plan: dry-run emits this; execute consumes it.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub nodes: Vec<ExecutionNode>,
    pub outputs: Vec<ArtifactOutput>,
    /// Legacy alias for single-node-per-recipe dry-run consumers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<PlannedRecipe>,
}

/// Legacy step shape (one entry per recipe when graph is single-node).
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

/// Build an [`ExecutionPlan`] without invoking runners.
pub fn plan(req: &PostprocessRequest) -> Result<ExecutionPlan, PostprocessError> {
    if req.recipes.is_empty() {
        return Err(PostprocessError::Usage("no recipes specified".into()));
    }
    if req.inputs.is_empty() {
        return Err(PostprocessError::Usage("no inputs specified".into()));
    }

    let output_dir = req
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    let mut nodes = Vec::new();
    let mut all_outputs = Vec::new();
    let mut steps = Vec::new();

    for recipe_path in &req.recipes {
        let (mut recipe_nodes, step) =
            plan_recipe(req, recipe_path, &output_dir, req.recipes.len() > 1)?;
        all_outputs.extend(recipe_nodes.iter().flat_map(|n| n.outputs.iter().cloned()));
        nodes.append(&mut recipe_nodes);
        steps.push(step);
    }

    Ok(ExecutionPlan {
        nodes,
        outputs: all_outputs,
        steps,
    })
}

fn plan_recipe(
    req: &PostprocessRequest,
    recipe_path: &Path,
    output_dir: &Path,
    multi_recipe: bool,
) -> Result<(Vec<ExecutionNode>, PlannedRecipe), PostprocessError> {
    let doc = recipe::load_recipe(recipe_path)?;
    validate_job_inputs(&doc, &req.inputs, recipe_path)?;

    let base_vars = build_plan_vars(&doc, req)?;
    let recipe_default = doc.runner.clone().unwrap_or_default();
    let ordered = topo_sort_nodes(&doc.graph)?;

    let mut nodes = Vec::new();
    let mut recipe_step_outputs = Vec::new();
    let mut recipe_step_body = String::new();

    for node in ordered {
        let plan_id = plan_node_id(doc.id.as_deref(), &node.id, multi_recipe);
        let mut runner = resolve_node_runner(&req.runner, &recipe_default, node.runner.as_ref())?;
        if runner.r#type.is_empty() {
            runner.r#type = "stub".into();
        }
        runner::validate_runner_type(&runner.r#type)?;

        let vars = enrich_node_vars(&base_vars, node, req)?;
        let body_src = node
            .prompt
            .as_deref()
            .or(node.command.as_deref())
            .unwrap_or("");
        let rendered_body = recipe::render_template(body_src, &vars);

        let node_outputs_decl = if node.outputs.is_empty() {
            &doc.outputs
        } else {
            &node.outputs
        };
        let mut outputs = Vec::new();
        for o in node_outputs_decl {
            let path =
                recipe::resolve_output_path(&o.resolved_path_pattern(), output_dir, &vars);
            let ao = ArtifactOutput {
                artifact: o.artifact.clone(),
                path,
                r#type: o.r#type.clone(),
                mime: o.mime.clone(),
            };
            recipe_step_outputs.push(PlannedOutput {
                id: ao.artifact.clone(),
                path: ao.path.clone(),
                mime: ao.mime.clone(),
                format: ao.r#type.clone(),
            });
            outputs.push(ao);
        }
        if recipe_step_body.is_empty() {
            recipe_step_body.clone_from(&rendered_body);
        }

        nodes.push(ExecutionNode {
            id: plan_id,
            graph_node_id: node.id.clone(),
            recipe_path: recipe_path.to_path_buf(),
            recipe_id: doc.id.clone(),
            runner,
            needs: node
                .needs
                .iter()
                .map(|n| plan_node_id(doc.id.as_deref(), n, multi_recipe))
                .collect(),
            rendered_body,
            outputs,
            parallel: node.needs.is_empty(),
        });
    }

    Ok((
        nodes,
        PlannedRecipe {
            recipe_path: recipe_path.to_path_buf(),
            recipe_id: doc.id.clone(),
            rendered_body: recipe_step_body,
            outputs: recipe_step_outputs,
        },
    ))
}

fn build_plan_vars(
    doc: &RecipeDoc,
    req: &PostprocessRequest,
) -> Result<BTreeMap<String, String>, PostprocessError> {
    let mut vars = doc.default_variables();
    vars.extend(req.variables.clone());
    for k in doc.secrets.keys() {
        vars.insert(k.clone(), format!("[secret:{k}]"));
    }
    for (name, binding) in &req.inputs {
        vars.insert(name.clone(), read_input(&binding.path)?);
    }
    Ok(vars)
}

fn enrich_node_vars(
    base: &BTreeMap<String, String>,
    node: &GraphNode,
    req: &PostprocessRequest,
) -> Result<BTreeMap<String, String>, PostprocessError> {
    let mut vars = base.clone();
    for (name, refer) in &node.inputs {
        match refer {
            InputRef::From { from } => {
                vars.entry(name.clone())
                    .or_insert_with(|| format!("[from:{from}]"));
            }
            InputRef::Artifact { artifact, .. } => {
                if let Some(binding) = req.inputs.get(artifact).or_else(|| {
                    req.inputs
                        .values()
                        .find(|b| b.artifact.as_deref() == Some(artifact.as_str()))
                }) {
                    vars.insert(name.clone(), read_input(&binding.path)?);
                } else if let Some(binding) = req.inputs.get(name) {
                    vars.insert(name.clone(), read_input(&binding.path)?);
                }
            }
        }
    }
    Ok(vars)
}

/// Plan then execute; returns registered derived artifacts.
pub fn execute(req: &PostprocessRequest) -> Result<PostprocessResult, PostprocessError> {
    execute_with_progress(req, None)
}

/// Callback: `(node_index, node_total, node)`.
pub type NodeProgressFn<'a> = dyn Fn(usize, usize, &ExecutionNode) + 'a;

/// Like [`execute`], with optional per-node progress hook.
pub fn execute_with_progress(
    req: &PostprocessRequest,
    on_node: Option<&NodeProgressFn<'_>>,
) -> Result<PostprocessResult, PostprocessError> {
    let plan = plan(req)?;
    let mut by_recipe: BTreeMap<PathBuf, Vec<&ExecutionNode>> = BTreeMap::new();
    for node in &plan.nodes {
        by_recipe
            .entry(node.recipe_path.clone())
            .or_default()
            .push(node);
    }

    let total = plan.nodes.len();
    let mut produced: HashMap<String, PathBuf> = HashMap::new();
    for (i, node) in plan.nodes.iter().enumerate() {
        if let Some(cb) = on_node {
            cb(i, total, node);
        }
        execute_node(req, node, &plan.nodes, &mut produced)?;
    }

    let mut results = Vec::new();
    for (_recipe_path, nodes) in by_recipe {
        let recipe_id = nodes.first().and_then(|n| n.recipe_id.clone());
        let mut outputs = Vec::new();
        let mut seen = HashSet::new();
        for n in nodes {
            for o in &n.outputs {
                if seen.insert(o.artifact.clone()) {
                    outputs.push(DerivedArtifact {
                        id: o.artifact.clone(),
                        path: o.path.clone(),
                    });
                }
            }
        }
        results.push(RecipeResult { recipe_id, outputs });
    }
    Ok(PostprocessResult { results })
}

fn execute_node(
    req: &PostprocessRequest,
    node: &ExecutionNode,
    all_nodes: &[ExecutionNode],
    produced: &mut HashMap<String, PathBuf>,
) -> Result<(), PostprocessError> {
    let doc = recipe::load_recipe(&node.recipe_path)?;
    let graph_node = doc
        .graph
        .iter()
        .find(|n| n.id == node.graph_node_id)
        .ok_or_else(|| {
            PostprocessError::Other(format!(
                "internal: missing graph node {}",
                node.graph_node_id
            ))
        })?;

    let backend = runner::resolve_runner(&node.runner)?;
    let mut vars = doc.default_variables();
    vars.extend(req.variables.clone());
    vars.extend(doc.resolve_secrets()?);
    for (name, binding) in &req.inputs {
        vars.insert(name.clone(), read_input(&binding.path)?);
    }
    for (name, refer) in &graph_node.inputs {
        match refer {
            InputRef::From { from } => {
                let path = resolve_from(from, produced, all_nodes)?;
                vars.insert(name.clone(), read_input(&path)?);
            }
            InputRef::Artifact { artifact, .. } => {
                if let Some(binding) = req.inputs.get(artifact) {
                    vars.insert(name.clone(), read_input(&binding.path)?);
                }
            }
        }
    }

    let body_src = graph_node
        .prompt
        .as_deref()
        .or(graph_node.command.as_deref())
        .unwrap_or("");
    let rendered_body = recipe::render_template(body_src, &vars);
    let outputs_decl: Vec<RecipeOutput> = if graph_node.outputs.is_empty() {
        doc.outputs.clone()
    } else {
        graph_node.outputs.clone()
    };
    let paths: Vec<PathBuf> = node.outputs.iter().map(|o| o.path.clone()).collect();
    for path in &paths {
        if path.exists() && !req.overwrite {
            return Err(PostprocessError::Usage(format!(
                "output exists (pass --overwrite): {}",
                path.display()
            )));
        }
    }

    backend.execute(&runner::RunnerInvoke {
        rendered_body: &rendered_body,
        outputs: &outputs_decl,
        output_paths: &paths,
        variables: &vars,
    })?;

    for o in &node.outputs {
        produced.insert(format!("{}.{}", node.id, o.artifact), o.path.clone());
        if let Some(raw_id) = node.id.rsplit('/').next() {
            produced.insert(format!("{raw_id}.{}", o.artifact), o.path.clone());
        }
        produced.insert(o.artifact.clone(), o.path.clone());
    }
    Ok(())
}

fn resolve_from(
    from: &str,
    produced: &HashMap<String, PathBuf>,
    nodes: &[ExecutionNode],
) -> Result<PathBuf, PostprocessError> {
    if let Some(p) = produced.get(from) {
        return Ok(p.clone());
    }
    // from: node.output
    if let Some((node, output)) = from.split_once('.') {
        let key = format!("{node}.{output}");
        if let Some(p) = produced.get(&key) {
            return Ok(p.clone());
        }
        // Try plan-prefixed ids
        for n in nodes {
            if n.id == node || n.id.ends_with(&format!("/{node}")) {
                let k = format!("{}.{}", n.id, output);
                if let Some(p) = produced.get(&k) {
                    return Ok(p.clone());
                }
            }
        }
    }
    Err(PostprocessError::Usage(format!(
        "input from '{from}' not produced yet (check needs)"
    )))
}

/// Resolve runner for a node: node pin, else `CLI/Job/Config override > Recipe default`.
fn resolve_node_runner(
    request_override: &RunnerSpec,
    recipe_default: &RunnerSpec,
    node_runner: Option<&RunnerSpec>,
) -> Result<RunnerSpec, PostprocessError> {
    if let Some(node) = node_runner {
        // Node pins — merge onto recipe default for sparse node specs; request override does NOT win.
        return Ok(recipe_default.merge_overlay(node));
    }
    // Inherit: request (CLI>Job>Config) over recipe default.
    Ok(recipe_default.merge_overlay(request_override))
}

fn plan_node_id(recipe_id: Option<&str>, node_id: &str, multi_recipe: bool) -> String {
    if multi_recipe {
        if let Some(rid) = recipe_id {
            return format!("{rid}/{node_id}");
        }
    }
    node_id.to_string()
}

fn topo_sort_nodes(graph: &[GraphNode]) -> Result<Vec<&GraphNode>, PostprocessError> {
    let id_set: HashSet<&str> = graph.iter().map(|n| n.id.as_str()).collect();
    let mut indeg: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for n in graph {
        indeg.entry(n.id.as_str()).or_insert(0);
        for dep in &n.needs {
            if !id_set.contains(dep.as_str()) {
                return Err(PostprocessError::Usage(format!(
                    "unknown needs '{dep}' on node '{}'",
                    n.id
                )));
            }
            *indeg.entry(n.id.as_str()).or_insert(0) += 1;
            adj.entry(dep.as_str()).or_default().push(n.id.as_str());
        }
    }
    let mut q: VecDeque<&str> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| *k)
        .collect();
    // Stable: sort roots by original order
    let order_index: HashMap<&str, usize> = graph
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut q_vec: Vec<&str> = std::mem::take(&mut q).into_iter().collect();
    q_vec.sort_by_key(|id| order_index.get(id).copied().unwrap_or(0));
    q.extend(q_vec);

    let mut out = Vec::with_capacity(graph.len());
    let by_id: HashMap<&str, &GraphNode> = graph.iter().map(|n| (n.id.as_str(), n)).collect();
    while let Some(id) = q.pop_front() {
        out.push(*by_id.get(id).unwrap());
        if let Some(nexts) = adj.get(id) {
            let mut ready = Vec::new();
            for n in nexts {
                if let Some(d) = indeg.get_mut(n) {
                    *d -= 1;
                    if *d == 0 {
                        ready.push(*n);
                    }
                }
            }
            ready.sort_by_key(|id| order_index.get(id).copied().unwrap_or(0));
            q.extend(ready);
        }
    }
    if out.len() != graph.len() {
        return Err(PostprocessError::Usage(
            "recipe graph has a cycle".into(),
        ));
    }
    Ok(out)
}

fn validate_job_inputs(
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
