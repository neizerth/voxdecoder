//! `vd-fix-layout remove`.

use std::io::{self, Write};

use crate::cli::{CliError, RemoveArgs};
use crate::models;
use crate::paths;

pub fn execute(args: RemoveArgs) -> Result<(), CliError> {
    let root = paths::resolve_models_dir(args.download_root.clone());
    if !args.yes {
        eprint!(
            "Remove pack '{}'? [y/N] ",
            models::resolve_model_name(&args.model)
        );
        let _ = io::stderr().flush();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| CliError::with_code(1, e.to_string()))?;
        let ok = matches!(line.trim(), "y" | "Y" | "yes" | "YES");
        if !ok {
            return Ok(());
        }
    }
    models::remove(&root, &args.model)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))
}
