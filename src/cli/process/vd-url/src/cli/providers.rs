//! `vd-url providers`.

use super::CliError;
use crate::provider::catalog_lines;

pub fn execute() -> Result<(), CliError> {
    for line in catalog_lines() {
        println!("{line}");
    }
    Ok(())
}
