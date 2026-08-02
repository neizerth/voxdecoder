//! `vd-url run`.

use super::{CliError, OutputFormat, RunArgs};
use crate::import::{resolve, UrlImportRequest};

pub fn execute(args: RunArgs) -> Result<(), CliError> {
    let request = UrlImportRequest {
        url: args.input.clone(),
        provider: args.provider.clone(),
        subtitles: args.subtitles,
        metadata_only: args.metadata_only,
        output_dir: args.output_dir.clone(),
        overwrite: args.overwrite,
    };
    let result = resolve(&request).map_err(CliError::from_import)?;
    report(&result, args.output, args.quiet)
}

pub fn report(
    result: &crate::import::ImportResult,
    format: OutputFormat,
    quiet: bool,
) -> Result<(), CliError> {
    if quiet {
        return Ok(());
    }
    match format {
        OutputFormat::Json => {
            let body = serde_json::to_string_pretty(&result.json_report())
                .map_err(|e| CliError::with_code(1, e.to_string()))?;
            println!("{body}");
        }
        OutputFormat::Text => {
            println!("provider: {}", result.provider);
            if let Some(a) = &result.audio {
                println!("audio: {}", a.path.display());
            }
            println!("metadata: {}", result.metadata.path.display());
            if let Some(s) = &result.subtitle {
                println!("subtitle: {}", s.path.display());
            }
        }
    }
    Ok(())
}
