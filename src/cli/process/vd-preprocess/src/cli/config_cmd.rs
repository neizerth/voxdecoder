//! `vd-preprocess config`.

use super::{CliError, ConfigAction, ConfigArgs};
use crate::config;
use crate::paths;

pub fn execute(args: ConfigArgs) -> Result<(), CliError> {
    let path = paths::config_path();
    match args.action {
        ConfigAction::Path => {
            println!("{}", path.display());
            Ok(())
        }
        ConfigAction::List => {
            let cfg = config::load(&path).map_err(CliError::usage)?;
            for line in cfg.list_lines() {
                println!("{line}");
            }
            Ok(())
        }
        ConfigAction::Get { key } => {
            let cfg = config::load(&path).map_err(CliError::usage)?;
            println!("{}", cfg.get(&key).map_err(CliError::usage)?);
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut cfg = config::load(&path).map_err(CliError::usage)?;
            cfg.set(&key, &value).map_err(CliError::usage)?;
            config::save(&path, &cfg).map_err(|e| CliError::with_code(1, e))?;
            Ok(())
        }
    }
}
