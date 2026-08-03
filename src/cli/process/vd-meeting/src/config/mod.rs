//! Config load / save.

mod file;

pub use file::{load, save};

use crate::model::{AlignmentMode, DiarizationEnabled};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub diarization_enabled: Option<String>,
    pub alignment_mode: Option<String>,
    pub asr: Option<String>,
    pub max_parallel: Option<u32>,
    pub progress: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    pub diarization_enabled: &'static str,
    pub alignment_mode: &'static str,
    pub progress: &'static str,
}

pub fn defaults() -> Defaults {
    Defaults {
        diarization_enabled: "auto",
        alignment_mode: "longest",
        progress: "text",
    }
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "diarization.enabled" => Ok(self
                .diarization_enabled
                .clone()
                .unwrap_or_else(|| d.diarization_enabled.to_string())),
            "alignment.mode" => Ok(self
                .alignment_mode
                .clone()
                .unwrap_or_else(|| d.alignment_mode.to_string())),
            "asr" => Ok(self.asr.clone().unwrap_or_default()),
            "max_parallel" => Ok(self.max_parallel.map(|n| n.to_string()).unwrap_or_default()),
            "progress" => Ok(self
                .progress
                .clone()
                .unwrap_or_else(|| d.progress.to_string())),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "diarization.enabled" => {
                if DiarizationEnabled::parse(value).is_none() {
                    return Err(format!("invalid diarization.enabled: {value}"));
                }
                self.diarization_enabled = Some(value.to_string());
            }
            "alignment.mode" => {
                if AlignmentMode::parse(value).is_none() {
                    return Err(format!("invalid alignment.mode: {value}"));
                }
                self.alignment_mode = Some(value.to_string());
            }
            "asr" => {
                self.asr = Some(value.to_string());
            }
            "max_parallel" => {
                let n: u32 = value
                    .parse()
                    .map_err(|_| format!("invalid max_parallel: {value}"))?;
                self.max_parallel = Some(n);
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
                "diarization.enabled = {}",
                self.diarization_enabled
                    .as_deref()
                    .unwrap_or(d.diarization_enabled)
            ),
            format!(
                "alignment.mode = {}",
                self.alignment_mode.as_deref().unwrap_or(d.alignment_mode)
            ),
            format!("asr = {}", self.asr.as_deref().unwrap_or("(pipeline)")),
            format!(
                "max_parallel = {}",
                self.max_parallel
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "(pipeline)".into())
            ),
            format!(
                "progress = {}",
                self.progress.as_deref().unwrap_or(d.progress)
            ),
        ]
    }
}
