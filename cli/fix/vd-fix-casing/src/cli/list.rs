//! `vd-fix-casing list`.

use crate::cli::{CliError, CliListFormat, ListArgs};
use crate::models;
use crate::paths;

pub fn execute(args: ListArgs) -> Result<(), CliError> {
    let root = paths::resolve_models_dir(args.download_root.clone());
    let rows = models::list_status(&root, args.all);
    if matches!(args.format, CliListFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows)
                .map_err(|e| CliError::with_code(1, e.to_string()))?
        );
        return Ok(());
    }
    println!("Models dir: {}", root.display());
    println!();
    for row in rows {
        let note = if row.installed {
            "ready"
        } else if row.shipping {
            "missing"
        } else {
            "missing (not shipping)"
        };
        println!("{:<2} {:<16} {note}", row.mark, row.name);
    }
    Ok(())
}
