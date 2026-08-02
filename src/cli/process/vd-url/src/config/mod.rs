//! Config load / save.

mod file;

pub use file::{load, save};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub provider: Option<String>,
    pub subtitles: Option<String>,
    pub progress: Option<String>,
}

pub fn defaults() -> Defaults {
    Defaults {
        provider: "auto",
        subtitles: "ignore",
        progress: "text",
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub provider: &'static str,
    pub subtitles: &'static str,
    pub progress: &'static str,
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "provider" => Ok(self
                .provider
                .clone()
                .unwrap_or_else(|| d.provider.to_string())),
            "subtitles" => Ok(self
                .subtitles
                .clone()
                .unwrap_or_else(|| d.subtitles.to_string())),
            "progress" => Ok(self
                .progress
                .clone()
                .unwrap_or_else(|| d.progress.to_string())),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "provider" => {
                self.provider = Some(value.to_string());
            }
            "subtitles" => {
                crate::import::SubtitlePolicy::parse(value)?;
                self.subtitles = Some(value.to_string());
            }
            "progress" => {
                if !matches!(value, "text" | "json") {
                    return Err(format!("invalid progress: {value}"));
                }
                self.progress = Some(value.to_string());
            }
            _ => return Err(format!("unknown config key: {key}")),
        }
        Ok(())
    }

    pub fn list_lines(&self) -> Vec<String> {
        let d = defaults();
        vec![
            format!(
                "provider = {}",
                self.provider.as_deref().unwrap_or(d.provider)
            ),
            format!(
                "subtitles = {}",
                self.subtitles.as_deref().unwrap_or(d.subtitles)
            ),
            format!(
                "progress = {}",
                self.progress.as_deref().unwrap_or(d.progress)
            ),
        ]
    }
}
