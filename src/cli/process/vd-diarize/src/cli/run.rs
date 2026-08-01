//! `vd-diarize run`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{CliError, ProgressMode};
use crate::backend::{BackendSpec, DiarizeRequest};
use crate::config;
use crate::paths;
use crate::run;

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub device: Option<String>,
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
    if !args.input.exists() {
        return Err(CliError::with_code(
            3,
            format!("input missing: {}", args.input.display()),
        ));
    }

    let file = config::load(&paths::config_path()).map_err(CliError::usage)?;
    let d = config::defaults();
    let provider = args
        .backend
        .clone()
        .or(file.provider.clone())
        .unwrap_or_else(|| d.provider.to_string());
    let model = args.model.clone().or(file.model.clone());
    let device = args.device.clone().or(file.device.clone());

    let req = DiarizeRequest {
        input: args.input.clone(),
        output: args.output.clone(),
        backend: BackendSpec::new(provider, model),
        device,
        options: BTreeMap::default(),
    };

    if args.dry_run {
        let plan = serde_json::json!({
            "input": req.input,
            "output": req.output.clone().unwrap_or_else(|| {
                crate::artifact::default_output_path(&req.input)
            }),
            "backend": {
                "provider": req.backend.provider,
                "model": req.backend.default_model(),
            },
            "device": req.device,
        });
        if args.json {
            println!("{}", serde_json::to_string_pretty(&plan).unwrap());
        } else {
            println!(
                "diarize  input={}  backend={}/{}  output={}",
                req.input.display(),
                req.backend.provider,
                req.backend.default_model(),
                plan["output"].as_str().unwrap_or("")
            );
        }
        return Ok(());
    }

    let out = run::diarize(
        &req,
        args.effective_progress(file.progress.as_deref()),
        args.overwrite,
    )
    .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
    println!("output: {}", out.output.display());
    Ok(())
}
