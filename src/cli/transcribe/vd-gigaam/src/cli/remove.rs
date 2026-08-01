//! `vd-gigaam remove`.

use std::io::{self, IsTerminal};

use crate::cli::{CliError, RemoveArgs};
use crate::gigaam::weights;
use crate::paths;

pub fn execute(args: RemoveArgs) -> Result<(), CliError> {
    let root = paths::resolve_models_dir(None);
    if !args.yes {
        if !io::stdin().is_terminal() {
            return Err(CliError::usage(
                "refusing to remove without --yes in non-interactive mode",
            ));
        }
        eprint!(
            "Remove model '{}' from {}? [y/N] ",
            args.model,
            root.display()
        );
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| CliError::with_code(1, e.to_string()))?;
        let ok = matches!(line.trim(), "y" | "Y" | "yes" | "YES");
        if !ok {
            return Err(CliError::usage("aborted"));
        }
    }
    weights::remove(&root, &args.model).map_err(|e| CliError::with_code(1, e.to_string()))?;
    Ok(())
}
