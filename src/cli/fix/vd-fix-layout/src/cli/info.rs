//! `vd-fix-layout info`.

use crate::cli::{CliError, InfoArgs};
use crate::models;
use crate::paths;

pub fn execute(args: InfoArgs) -> Result<(), CliError> {
    let root = paths::resolve_models_dir(args.download_root.clone());
    let info = models::info(&root, &args.model)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&info)
                .map_err(|e| CliError::with_code(1, e.to_string()))?
        );
        return Ok(());
    }
    println!("name:       {}", info.name);
    println!("language:   {}", info.language);
    println!("backend:    {}", info.backend);
    println!("version:    {}", info.version);
    println!("installed:  {}", if info.installed { "yes" } else { "no" });
    println!("path:       {}", info.path.as_deref().unwrap_or("—"));
    if let Some(size) = info.size {
        println!("size:       {size} B");
    }
    Ok(())
}
