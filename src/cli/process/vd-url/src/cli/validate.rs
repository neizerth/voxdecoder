//! `vd-url validate` — offline checks.

use super::{CliError, ValidateArgs};
use crate::import::validate_request;

pub fn execute(args: ValidateArgs) -> Result<(), CliError> {
    let id = validate_request(
        &args.input,
        args.provider.as_deref(),
        args.subtitles,
    )
    .map_err(CliError::from_import)?;
    println!("✓ URL valid");
    println!("✓ Provider resolved ({id})");
    println!("✓ Subtitles policy supported");
    Ok(())
}
