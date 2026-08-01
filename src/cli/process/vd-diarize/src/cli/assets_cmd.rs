//! Asset pack commands.

use super::CliError;
use crate::assets;

pub fn install(provider: &str) -> Result<(), CliError> {
    let dir = assets::install(provider).map_err(CliError::usage)?;
    println!("installed {provider} → {}", dir.display());
    Ok(())
}

pub fn remove(provider: &str) -> Result<(), CliError> {
    assets::remove(provider).map_err(CliError::usage)?;
    println!("removed {provider}");
    Ok(())
}

pub fn list() -> Result<(), CliError> {
    println!("assets root: {}", assets::assets_root_display().display());
    for p in assets::list_installed() {
        println!("{p}");
    }
    Ok(())
}

pub fn info(provider: &str) -> Result<(), CliError> {
    let m = assets::info(provider).map_err(CliError::usage)?;
    println!("provider = {}", m.provider);
    println!("version = {}", m.version);
    println!("status = {}", m.status);
    if !m.notes.is_empty() {
        println!("notes = {}", m.notes);
    }
    Ok(())
}
