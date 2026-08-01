//! Config load / save.

mod file;

pub use file::{load, save};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub progress: Option<String>,
    pub asr: Option<String>,
    pub continue_on_error: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub progress: &'static str,
    pub asr: &'static str,
    pub continue_on_error: bool,
}

pub fn defaults() -> Defaults {
    Defaults {
        progress: "text",
        asr: "gigaam",
        continue_on_error: false,
    }
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "progress" => Ok(self
                .progress
                .clone()
                .unwrap_or_else(|| d.progress.to_string())),
            "asr" => Ok(self.asr.clone().unwrap_or_else(|| d.asr.to_string())),
            "continue_on_error" => {
                Ok(bool_str(self.continue_on_error.unwrap_or(d.continue_on_error)).to_string())
            }
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "progress" => {
                if !matches!(value, "text" | "json") {
                    return Err(format!("invalid progress: {value}"));
                }
                self.progress = Some(value.to_string());
            }
            "asr" => {
                if crate::job::TranscribeEngine::parse(value).is_none() {
                    return Err(format!("invalid asr: {value}"));
                }
                self.asr = Some(value.to_string());
            }
            "continue_on_error" => {
                self.continue_on_error = Some(parse_bool(value)?);
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
                self.progress.as_deref().unwrap_or(d.progress)
            ),
            format!("asr = {}", self.asr.as_deref().unwrap_or(d.asr)),
            format!(
                "continue_on_error = {}",
                bool_str(self.continue_on_error.unwrap_or(d.continue_on_error))
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
