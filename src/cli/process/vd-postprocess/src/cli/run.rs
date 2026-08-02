//! `vd-postprocess run`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use super::{CliError, ProgressMode};
use crate::config;
use crate::paths;
use crate::postprocess::{self, ArtifactBinding, PostprocessRequest, RunnerSpec};
use crate::status;

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub inputs: Vec<String>,
    pub input_file: Option<PathBuf>,
    pub recipes: Vec<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub runner: Option<String>,
    pub model: Option<String>,
    pub vars: Vec<String>,
    pub dry_run: bool,
    pub json: bool,
    pub progress: Option<ProgressMode>,
    pub quiet: bool,
    pub overwrite: bool,
}

impl RunArgs {
    fn effective_progress(&self, file: Option<&str>) -> ProgressMode {
        if self.quiet {
            return ProgressMode::None;
        }
        if let Some(p) = self.progress {
            return p;
        }
        file.and_then(ProgressMode::parse)
            .unwrap_or(ProgressMode::Text)
    }
}

pub fn execute(args: RunArgs) -> Result<(), CliError> {
    let file_cfg = config::load(&paths::config_path()).map_err(CliError::usage)?;

    let mut inputs = BTreeMap::new();
    for spec in &args.inputs {
        let (name, path, artifact) = parse_input(spec)?;
        inputs.insert(
            name,
            ArtifactBinding {
                path: PathBuf::from(path),
                artifact,
                format: None,
            },
        );
    }
    if let Some(path) = &args.input_file {
        inputs.entry("input".into()).or_insert_with(|| ArtifactBinding {
            path: path.clone(),
            artifact: Some("input".into()),
            format: None,
        });
    }
    if inputs.is_empty() {
        return Err(CliError::usage("no inputs specified"));
    }

    let mut variables = BTreeMap::new();
    for spec in &args.vars {
        let (k, v) = parse_kv(spec, "var")?;
        variables.insert(k, v);
    }

    // CLI > Config > Recipe (Recipe applied in plan). Empty type ⇒ leave for recipe / stub fallback.
    let runner_type = args
        .runner
        .clone()
        .or(file_cfg.runner_type.clone())
        .unwrap_or_default();
    let model = args.model.clone().or(file_cfg.runner_model.clone());

    let req = PostprocessRequest {
        inputs,
        recipes: args.recipes.clone(),
        runner: RunnerSpec {
            r#type: runner_type.clone(),
            model: model.clone(),
            ..Default::default()
        },
        variables,
        output_dir: args.output_dir.clone(),
        overwrite: args.overwrite,
    };

    let progress = status::start(args.effective_progress(file_cfg.progress.as_deref()));
    let runner_label = if runner_type.is_empty() {
        model.as_deref().unwrap_or("recipe-graph")
    } else {
        runner_type.as_str()
    };
    status::emit_start(&progress, req.recipes.len(), Some(runner_label));
    status::emit_phase(&progress, "planning", 10);

    if args.dry_run {
        let plan = postprocess::plan(&req)
            .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
        status::emit_phase(&progress, "planned", 100);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&plan).unwrap());
        } else {
            println!("{}", serde_yaml::to_string(&plan).unwrap());
        }
        return Ok(());
    }

    let started = Instant::now();
    let result = postprocess::execute_with_progress(
        &req,
        Some(&|index, total, _node| {
            let pct = (70usize)
                .checked_mul(index + 1)
                .and_then(|n| n.checked_div(total))
                .map(|n| 20 + n)
                .and_then(|n| u8::try_from(n).ok())
                .unwrap_or(50)
                .min(90);
            status::emit_node(
                &progress,
                "executing",
                pct,
                u32::try_from(index + 1).unwrap_or(1),
                u32::try_from(total).unwrap_or(1),
            );
        }),
    )
    .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;

    let primary = result
        .results
        .first()
        .and_then(|r| r.outputs.first())
        .map(|o| o.path.as_path());
    status::emit_done(&progress, primary, started.elapsed().as_secs_f64());

    for rr in &result.results {
        for o in &rr.outputs {
            println!("{}: {}", o.id, o.path.display());
        }
    }
    Ok(())
}

fn parse_input(spec: &str) -> Result<(String, String, Option<String>), CliError> {
    // name.artifact=path  OR  name=path
    let (k, v) = spec.split_once('=').ok_or_else(|| {
        CliError::usage(format!("bad --input (expected name=path): {spec}"))
    })?;
    let k = k.trim();
    let v = v.trim();
    if k.is_empty() || v.is_empty() {
        return Err(CliError::usage(format!("bad --input: {spec}")));
    }
    if let Some(name) = k.strip_suffix(".artifact") {
        return Ok((name.to_string(), v.to_string(), Some(v.to_string())));
    }
    Ok((k.to_string(), v.to_string(), Some(k.to_string())))
}

fn parse_kv(spec: &str, kind: &str) -> Result<(String, String), CliError> {
    let (k, v) = spec.split_once('=').ok_or_else(|| {
        CliError::usage(format!("bad --{kind} (expected key=value): {spec}"))
    })?;
    let k = k.trim();
    let v = v.trim();
    if k.is_empty() || v.is_empty() {
        return Err(CliError::usage(format!("bad --{kind}: {spec}")));
    }
    Ok((k.to_string(), v.to_string()))
}
