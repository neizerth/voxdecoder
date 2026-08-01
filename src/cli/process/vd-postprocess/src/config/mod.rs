//! Config load / save.

mod file;

pub use file::{load, save};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub provider_type: Option<String>,
    pub provider_model: Option<String>,
    pub progress: Option<String>,
}

pub fn defaults() -> Defaults {
    Defaults {
        provider_type: "stub",
        progress: "text",
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub provider_type: &'static str,
    pub progress: &'static str,
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "provider.type" => Ok(self
                .provider_type
                .clone()
                .unwrap_or_else(|| d.provider_type.to_string())),
            "provider.model" => Ok(self.provider_model.clone().unwrap_or_default()),
            "progress" => Ok(self
                .progress
                .clone()
                .unwrap_or_else(|| d.progress.to_string())),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "provider.type" => {
                self.provider_type = Some(value.to_string());
            }
            "provider.model" => {
                self.provider_model = Some(value.to_string());
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
                "provider.type = {}",
                self.provider_type.as_deref().unwrap_or(d.provider_type)
            ),
            format!(
                "provider.model = {}",
                self.provider_model.as_deref().unwrap_or("(none)")
            ),
            format!(
                "progress = {}",
                self.progress.as_deref().unwrap_or(d.progress)
            ),
        ]
    }
}
