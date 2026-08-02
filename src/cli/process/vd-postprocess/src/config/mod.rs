//! Config load / save.

mod file;

pub use file::{load, save};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub runner_type: Option<String>,
    pub runner_model: Option<String>,
    pub progress: Option<String>,
}

pub fn defaults() -> Defaults {
    Defaults {
        runner_type: "stub",
        progress: "text",
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub runner_type: &'static str,
    pub progress: &'static str,
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "runner.type" | "provider.type" => Ok(self
                .runner_type
                .clone()
                .unwrap_or_else(|| d.runner_type.to_string())),
            "runner.model" | "provider.model" => {
                Ok(self.runner_model.clone().unwrap_or_default())
            }
            "progress" => Ok(self
                .progress
                .clone()
                .unwrap_or_else(|| d.progress.to_string())),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "runner.type" | "provider.type" => {
                self.runner_type = Some(value.to_string());
            }
            "runner.model" | "provider.model" => {
                self.runner_model = Some(value.to_string());
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
                "runner.type = {}",
                self.runner_type.as_deref().unwrap_or(d.runner_type)
            ),
            format!(
                "runner.model = {}",
                self.runner_model.as_deref().unwrap_or("(none)")
            ),
            format!(
                "progress = {}",
                self.progress.as_deref().unwrap_or(d.progress)
            ),
        ]
    }
}
