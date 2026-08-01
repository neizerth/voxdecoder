//! `vd-giga config` subcommands.

use crate::cli::{CliError, ConfigAction, ConfigArgs};
use crate::config::file as config_file;
use crate::paths;

pub fn execute(args: ConfigArgs) -> Result<(), CliError> {
    let path = paths::config_path();
    match args.action {
        ConfigAction::Path => {
            println!("{}", path.display());
            Ok(())
        }
        ConfigAction::List => {
            let cfg = config_file::load(&path).map_err(CliError::usage)?;
            for line in cfg.list_lines() {
                println!("{line}");
            }
            Ok(())
        }
        ConfigAction::Get { key } => {
            let cfg = config_file::load(&path).map_err(CliError::usage)?;
            let defaults = crate::config::resolve::defaults();
            let value = match key.as_str() {
                "model" => cfg.model.unwrap_or(defaults.model),
                "device" => cfg.device.unwrap_or(defaults.device).as_str().to_string(),
                "fp16_encoder" => on_off(cfg.fp16_encoder.unwrap_or(defaults.fp16_encoder)),
                "flash" => on_off(cfg.flash.unwrap_or(defaults.flash)),
                "download_root" => cfg.download_root.unwrap_or_default(),
                "word_timestamps" => {
                    on_off(cfg.word_timestamps.unwrap_or(defaults.word_timestamps))
                }
                "format" => cfg.format.unwrap_or(defaults.format).as_str().to_string(),
                other => {
                    return Err(CliError::usage(format!("unknown config key '{other}'")));
                }
            };
            println!("{value}");
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut cfg = config_file::load(&path).map_err(CliError::usage)?;
            cfg.set(&key, &value).map_err(CliError::usage)?;
            config_file::save(&path, &cfg).map_err(CliError::usage)?;
            Ok(())
        }
    }
}

fn on_off(v: bool) -> String {
    if v { "on" } else { "off" }.to_string()
}
