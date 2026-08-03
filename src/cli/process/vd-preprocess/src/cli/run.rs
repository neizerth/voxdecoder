//! `vd-preprocess run`.

use std::path::PathBuf;

use super::{CliError, ProgressMode};
use crate::config;
use crate::paths;
use crate::preprocess::{
    self, load_chain_file, parse_filter_flag, request_from_raw, RawFilter,
};
use crate::status;

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: PathBuf,
    pub chain: Option<PathBuf>,
    pub filters: Vec<String>,
    pub provider: Option<String>,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
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

    let mut chain_provider = None;
    let mut raw: Vec<RawFilter> = Vec::new();
    if let Some(chain_path) = &args.chain {
        let chain = load_chain_file(chain_path)
            .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
        chain_provider = chain.provider;
        raw.extend(chain.filters);
    }

    // CLI > config > chain file > default
    let default_provider = args
        .provider
        .clone()
        .or(file_cfg.provider.clone())
        .or(chain_provider)
        .unwrap_or_else(|| d.provider.to_string());

    for flag in &args.filters {
        let spec = parse_filter_flag(flag, &default_provider)
            .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
        raw.push(RawFilter {
            provider: Some(spec.provider),
            operation: Some(spec.operation),
            r#type: None,
            params: spec.params,
        });
    }

    let req = request_from_raw(
        args.input.clone(),
        raw,
        &default_provider,
        args.output.clone(),
        args.output_dir.clone(),
        args.overwrite,
    )
    .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;

    let progress = status::start(args.effective_progress(file_cfg.progress.as_deref()));
    status::emit_phase(&progress, "planning", 5);

    if args.dry_run {
        let plan = preprocess::plan(&req)
            .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&plan).unwrap());
        } else {
            println!("{}", serde_yaml::to_string(&plan).unwrap());
        }
        return Ok(());
    }

    status::emit_phase(&progress, "executing", 10);
    let result = preprocess::execute_with_progress(&req, Some(&progress))
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
    status::emit_phase(&progress, "done", 100);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        println!("{}", result.output.path.display());
        if let Some(tm) = &result.timemap {
            eprintln!("timemap: {}", tm.display());
        }
    }
    Ok(())
}
