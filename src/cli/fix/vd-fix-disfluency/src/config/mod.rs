//! Config load / save / merge.

mod file;
pub mod resolve;

use crate::types::{Language, Mode, ProgressFormat};

pub use file::{load, save};
pub use resolve::{resolve_run, DryRunPlan, RunOverrides};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub language: Option<Language>,
    pub mode: Option<Mode>,
    pub remove_fillers: Option<bool>,
    pub in_place: Option<bool>,
    pub progress: Option<ProgressFormat>,
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub language: Language,
    pub mode: Mode,
    pub remove_fillers: bool,
    pub in_place: bool,
    pub progress: ProgressFormat,
}

pub fn defaults() -> Defaults {
    Defaults {
        language: Language::Ru,
        mode: Mode::Light,
        remove_fillers: true,
        in_place: false,
        progress: ProgressFormat::Text,
    }
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "language" => Ok(self.language.unwrap_or(d.language).as_str().to_string()),
            "mode" => Ok(self.mode.unwrap_or(d.mode).as_str().to_string()),
            "remove_fillers" => {
                Ok(bool_str(self.remove_fillers.unwrap_or(d.remove_fillers)).to_string())
            }
            "in_place" => Ok(bool_str(self.in_place.unwrap_or(d.in_place)).to_string()),
            "progress" => Ok(self.progress.unwrap_or(d.progress).as_str().to_string()),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "language" => {
                self.language = Some(
                    Language::parse(value).ok_or_else(|| format!("invalid language: {value}"))?,
                );
            }
            "mode" => {
                self.mode =
                    Some(Mode::parse(value).ok_or_else(|| format!("invalid mode: {value}"))?);
            }
            "remove_fillers" => {
                self.remove_fillers = Some(parse_bool(value)?);
            }
            "in_place" => {
                self.in_place = Some(parse_bool(value)?);
            }
            "progress" => {
                self.progress = Some(
                    ProgressFormat::parse(value)
                        .ok_or_else(|| format!("invalid progress: {value}"))?,
                );
            }
            _ => return Err(format!("unknown config key: {key}")),
        }
        Ok(())
    }

    pub fn list_lines(&self) -> Vec<String> {
        let d = defaults();
        vec![
            format!(
                "language = {}",
                self.language.unwrap_or(d.language).as_str()
            ),
            format!("mode = {}", self.mode.unwrap_or(d.mode).as_str()),
            format!(
                "remove_fillers = {}",
                bool_str(self.remove_fillers.unwrap_or(d.remove_fillers))
            ),
            format!(
                "in_place = {}",
                bool_str(self.in_place.unwrap_or(d.in_place))
            ),
            format!(
                "progress = {}",
                self.progress.unwrap_or(d.progress).as_str()
            ),
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
