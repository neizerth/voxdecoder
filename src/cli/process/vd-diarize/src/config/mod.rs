//! Config load / save.

mod file;

pub use file::{load, save};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub progress: Option<String>,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub provider: &'static str,
    pub model: &'static str,
    pub progress: &'static str,
}

pub fn defaults() -> Defaults {
    Defaults {
        // Ship with working local backend; switch to pyannote when runtime lands.
        provider: "stub",
        model: "deterministic-v1",
        progress: "text",
    }
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "backend.provider" | "provider" => Ok(self
                .provider
                .clone()
                .unwrap_or_else(|| d.provider.to_string())),
            "backend.model" | "model" => Ok(self
                .model
                .clone()
                .unwrap_or_else(|| d.model.to_string())),
            "progress" => Ok(self
                .progress
                .clone()
                .unwrap_or_else(|| d.progress.to_string())),
            "device" => Ok(self.device.clone().unwrap_or_default()),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "backend.provider" | "provider" => {
                if !crate::backend::known_providers().contains(&value) {
                    return Err(format!("unknown provider: {value}"));
                }
                self.provider = Some(value.to_string());
            }
            "backend.model" | "model" => {
                self.model = Some(value.to_string());
            }
            "progress" => {
                if !matches!(value, "text" | "json") {
                    return Err(format!("invalid progress: {value}"));
                }
                self.progress = Some(value.to_string());
            }
            "device" => {
                self.device = Some(value.to_string());
            }
            _ => return Err(format!("unknown config key: {key}")),
        }
        Ok(())
    }

    pub fn list_lines(&self) -> Vec<String> {
        let d = defaults();
        vec![
            format!(
                "backend.provider = {}",
                self.provider.as_deref().unwrap_or(d.provider)
            ),
            format!(
                "backend.model = {}",
                self.model.as_deref().unwrap_or(d.model)
            ),
            format!(
                "progress = {}",
                self.progress.as_deref().unwrap_or(d.progress)
            ),
            format!(
                "device = {}",
                self.device.as_deref().unwrap_or("(auto)")
            ),
        ]
    }
}
