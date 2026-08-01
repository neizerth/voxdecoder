//! Config for `vd-assets`.

mod file;

use crate::types::ProgressFormat;

pub use file::{load, save};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub progress: Option<ProgressFormat>,
    pub ocr: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub progress: ProgressFormat,
    pub ocr: bool,
}

pub fn defaults() -> Defaults {
    Defaults {
        progress: ProgressFormat::Text,
        ocr: false,
    }
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "progress" => Ok(self.progress.unwrap_or(d.progress).as_str().to_string()),
            "ocr" => Ok(bool_str(self.ocr.unwrap_or(d.ocr)).to_string()),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "progress" => {
                self.progress = Some(
                    ProgressFormat::parse(value)
                        .ok_or_else(|| format!("invalid progress: {value}"))?,
                );
            }
            "ocr" => {
                self.ocr = Some(parse_bool(value)?);
            }
            _ => return Err(format!("unknown config key: {key}")),
        }
        Ok(())
    }

    pub fn list_lines(&self) -> Vec<String> {
        let d = defaults();
        vec![
            format!(
                "progress = {}",
                self.progress.unwrap_or(d.progress).as_str()
            ),
            format!("ocr = {}", bool_str(self.ocr.unwrap_or(d.ocr))),
        ]
    }
}

fn bool_str(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        _ => Err(format!("expected on|off, got {s}")),
    }
}
