//! `vd-postprocess run`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{CliError, ProgressMode};
use crate::config;
use crate::paths;
use crate::postprocess::{
    self, ArtifactBinding, ExecutionProviderSpec, PostprocessRequest,
};
use crate::status;

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub inputs: Vec<String>,
    pub input_file: Option<PathBuf>,
    pub recipes: Vec<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub provider: Option<String>,
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
    let d = config::defaults();

    let mut inputs = BTreeMap::new();
    for spec in &args.inputs {
        let (name, path) = parse_kv(spec, "input")?;
        inputs.insert(
            name,
            ArtifactBinding {
                path: PathBuf::from(path),
            },
        );
    }
    if let Some(path) = &args.input_file {
        inputs
            .entry("input".into())
            .or_insert_with(|| ArtifactBinding { path: path.clone() });
    }
    if inputs.is_empty() {
        return Err(CliError::usage("no inputs specified"));
    }

    let mut variables = BTreeMap::new();
    for spec in &args.vars {
        let (k, v) = parse_kv(spec, "var")?;
        variables.insert(k, v);
    }

    let provider_type = args
        .provider
        .clone()
        .or(file_cfg.provider_type.clone())
        .unwrap_or_else(|| d.provider_type.to_string());
    let model = args.model.clone().or(file_cfg.provider_model.clone());

    let req = PostprocessRequest {
        inputs,
        recipes: args.recipes.clone(),
        provider: ExecutionProviderSpec {
            r#type: provider_type,
            model,
            command: None,
            options: BTreeMap::new(),
        },
        variables,
        output_dir: args.output_dir.clone(),
        overwrite: args.overwrite,
    };

    let progress = status::start(args.effective_progress(file_cfg.progress.as_deref()));
    status::emit_phase(&progress, "planning", 20);

    if args.dry_run {
        let plan = postprocess::plan(&req)
            .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&plan).unwrap());
        } else {
            println!("{}", serde_yaml::to_string(&plan).unwrap());
        }
        return Ok(());
    }

    status::emit_phase(&progress, "executing", 60);
    let result = postprocess::execute(&req)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
    status::emit_phase(&progress, "done", 100);
    for rr in &result.results {
        for o in &rr.outputs {
            println!("{}: {}", o.id, o.path.display());
        }
    }
    Ok(())
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
