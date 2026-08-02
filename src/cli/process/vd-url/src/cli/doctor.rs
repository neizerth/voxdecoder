//! `vd-url doctor`.

use super::CliError;
use crate::provider::doctor_report;

pub fn execute() -> Result<(), CliError> {
    let checks = doctor_report();
    let mut all_ok = true;
    for c in &checks {
        let mark = if c.ok { "✓" } else { "✗" };
        println!("{mark} {:<8} {}", c.name, c.detail);
        all_ok &= c.ok;
    }
    if all_ok {
        Ok(())
    } else {
        Err(CliError::with_code(1, "doctor: missing tools"))
    }
}
