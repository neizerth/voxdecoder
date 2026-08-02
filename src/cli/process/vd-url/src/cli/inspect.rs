//! `vd-url inspect`.

use super::run::report;
use super::{CliError, InspectArgs};
use crate::import::{resolve, SubtitlePolicy, UrlImportRequest};

pub fn execute(args: InspectArgs) -> Result<(), CliError> {
    let request = UrlImportRequest {
        url: args.input.clone(),
        provider: args.provider.clone(),
        subtitles: SubtitlePolicy::Ignore,
        metadata_only: true,
        output_dir: args.output_dir.clone(),
        overwrite: args.overwrite,
    };
    let result = resolve(&request).map_err(CliError::from_import)?;
    report(&result, args.output, args.quiet)
}
