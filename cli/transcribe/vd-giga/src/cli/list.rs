//! `vd-giga list`.

use serde::Serialize;

use crate::cli::{CliError, ListArgs};
use crate::gigaam::catalog::CATALOG;
use crate::gigaam::weights;
use crate::paths;

#[derive(Serialize)]
struct ListItem {
    name: String,
    installed: bool,
}

pub fn execute(args: ListArgs) -> Result<(), CliError> {
    let root = paths::default_models_dir();
    let mut items: Vec<ListItem> = CATALOG
        .iter()
        .map(|name| ListItem {
            name: (*name).to_string(),
            installed: weights::is_installed(&root, name),
        })
        .collect();

    if !args.all {
        items.retain(|i| i.installed);
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_default()
        );
        return Ok(());
    }

    if !args.all {
        println!("Installed\n");
    }
    for item in items {
        let mark = if item.installed { "✓" } else { "○" };
        println!("{mark} {}", item.name);
    }
    Ok(())
}
