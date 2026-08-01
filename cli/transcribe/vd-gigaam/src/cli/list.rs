//! `vd-gigaam list`.

use serde::Serialize;

use crate::cli::{CliError, CliListFormat, ListArgs};
use crate::gigaam::catalog::CATALOG;
use crate::gigaam::weights::{self, ModelKind};
use crate::paths;

#[derive(Serialize)]
struct ListItem {
    name: String,
    /// Ready for run (converted SafeTensors in models dir).
    installed: bool,
    /// Converted | managed_ckpt | gigaam_cache | missing
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Serialize)]
struct ListJson {
    models_dir: String,
    /// Preferred GigaAM cache (same as models_dir when using defaults).
    gigaam_cache: String,
    models: Vec<ListItem>,
}

fn status_label(kind: ModelKind) -> &'static str {
    match kind {
        ModelKind::Converted => "converted",
        ModelKind::ManagedCkpt => "managed_ckpt",
        ModelKind::GigaamCache => "gigaam_cache",
        ModelKind::Missing => "missing",
    }
}

pub fn execute(args: ListArgs) -> Result<(), CliError> {
    let root = paths::resolve_models_dir(None);
    let gigaam = paths::preferred_gigaam_cache();
    let mut items: Vec<ListItem> = CATALOG
        .iter()
        .map(|name| {
            let st = weights::model_status(&root, name);
            ListItem {
                name: (*name).to_string(),
                installed: matches!(st.kind, ModelKind::Converted),
                status: status_label(st.kind),
                path: st.path.map(|p| p.display().to_string()),
            }
        })
        .collect();

    if !args.all {
        items.retain(|i| i.status != "missing");
    }

    if matches!(args.format, CliListFormat::Json) {
        let payload = ListJson {
            models_dir: root.display().to_string(),
            gigaam_cache: gigaam.display().to_string(),
            models: items,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
        return Ok(());
    }

    println!("Models dir: {}", root.display());
    if root != gigaam {
        println!("GigaAM cache: {}", gigaam.display());
    }
    println!();

    if !args.all {
        println!("Available\n");
        if items.is_empty() {
            println!("(none)");
            println!(
                "\nHint: vd-gigaam install MODEL   or set VD_GIGAAM_MODELS_DIR / download_root"
            );
            return Ok(());
        }
    }

    for item in items {
        let mark = match item.status {
            "converted" => "✓",
            "managed_ckpt" | "gigaam_cache" => "·",
            _ => "○",
        };
        let where_ = match item.status {
            "converted" => "ready",
            "managed_ckpt" => "ckpt",
            "gigaam_cache" => "ckpt (GigaAM cache)",
            _ => "missing",
        };
        println!("{mark} {:<14}  {where_}", item.name);
    }
    Ok(())
}
