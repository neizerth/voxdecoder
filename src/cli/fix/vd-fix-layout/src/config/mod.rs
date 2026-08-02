//! Config load / save / merge.

mod file;
pub mod resolve;

use crate::types::{Language, ParagraphDensity, ProgressFormat};

pub use file::{load, save};
pub use resolve::{resolve_run, DryRunPlan, RunOverrides};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub language: Option<Language>,
    pub paragraph_density: Option<ParagraphDensity>,
    pub use_timemap: Option<bool>,
    pub in_place: Option<bool>,
    pub progress: Option<ProgressFormat>,
    pub download_root: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub language: Language,
    pub paragraph_density: ParagraphDensity,
    pub use_timemap: bool,
    pub in_place: bool,
    pub progress: ProgressFormat,
}

pub fn defaults() -> Defaults {
    Defaults {
        language: Language::Auto,
        paragraph_density: ParagraphDensity::Normal,
        use_timemap: true,
        in_place: false,
        progress: ProgressFormat::Text,
    }
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "language" => Ok(self.language.unwrap_or(d.language).as_str().to_string()),
            "paragraph_density" => Ok(self
                .paragraph_density
                .unwrap_or(d.paragraph_density)
                .as_str()
                .to_string()),
            "use_timemap" => Ok(bool_str(self.use_timemap.unwrap_or(d.use_timemap)).to_string()),
            "in_place" => Ok(bool_str(self.in_place.unwrap_or(d.in_place)).to_string()),
            "progress" => Ok(self.progress.unwrap_or(d.progress).as_str().to_string()),
            "download_root" => Ok(self.download_root.clone().unwrap_or_default()),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "language" => {
                self.language = Some(parse_layout_language(value)?);
            }
            "paragraph_density" => {
                self.paragraph_density = Some(
                    ParagraphDensity::parse(value)
                        .ok_or_else(|| format!("invalid paragraph_density: {value}"))?,
                );
            }
            "use_timemap" => {
                self.use_timemap = Some(parse_bool(value)?);
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
            "download_root" => {
                self.download_root = Some(value.to_string());
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
            format!(
                "paragraph_density = {}",
                self.paragraph_density
                    .unwrap_or(d.paragraph_density)
                    .as_str()
            ),
            format!(
                "use_timemap = {}",
                bool_str(self.use_timemap.unwrap_or(d.use_timemap))
            ),
            format!(
                "in_place = {}",
                bool_str(self.in_place.unwrap_or(d.in_place))
            ),
            format!(
                "progress = {}",
                self.progress.unwrap_or(d.progress).as_str()
            ),
            format!(
                "download_root = {}",
                self.download_root.as_deref().unwrap_or("")
            ),
        ]
    }
}

/// Shipping languages for this CLI: `ru` / `en` / `auto` only.
pub fn parse_layout_language(s: &str) -> Result<Language, String> {
    match s {
        "ru" => Ok(Language::Ru),
        "en" => Ok(Language::En),
        "auto" => Ok(Language::Auto),
        _ => Err(format!(
            "invalid language: {s} (allowed: ru, en, auto)"
        )),
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
