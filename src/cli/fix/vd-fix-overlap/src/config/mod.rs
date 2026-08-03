//! Config load / save / merge (CLI > config > default).

mod file;

pub use file::{load, save};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileConfig {
    pub similarity_threshold: Option<f64>,
    pub max_gap_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Defaults {
    pub similarity_threshold: f64,
    pub max_gap_ms: u64,
}

pub fn defaults() -> Defaults {
    let d = crate::overlap::DetectOptions::default();
    Defaults {
        similarity_threshold: d.similarity_threshold,
        max_gap_ms: d.max_gap_ms,
    }
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Result<String, String> {
        let d = defaults();
        match key {
            "similarity_threshold" => Ok(self
                .similarity_threshold
                .unwrap_or(d.similarity_threshold)
                .to_string()),
            "max_gap_ms" => Ok(self.max_gap_ms.unwrap_or(d.max_gap_ms).to_string()),
            _ => Err(format!("unknown config key: {key}")),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "similarity_threshold" => {
                let v: f64 = value
                    .parse()
                    .map_err(|_| format!("invalid similarity_threshold: {value}"))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(format!(
                        "similarity_threshold must be in [0.0, 1.0], got {value}"
                    ));
                }
                self.similarity_threshold = Some(v);
            }
            "max_gap_ms" => {
                self.max_gap_ms = Some(
                    value
                        .parse()
                        .map_err(|_| format!("invalid max_gap_ms: {value}"))?,
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
                "similarity_threshold = {}",
                self.similarity_threshold.unwrap_or(d.similarity_threshold)
            ),
            format!("max_gap_ms = {}", self.max_gap_ms.unwrap_or(d.max_gap_ms)),
        ]
    }

    /// Merge CLI overrides > config file > defaults into a `DetectOptions`.
    pub fn resolve(
        &self,
        similarity_threshold: Option<f64>,
        max_gap_ms: Option<u64>,
    ) -> crate::overlap::DetectOptions {
        let d = defaults();
        crate::overlap::DetectOptions {
            similarity_threshold: similarity_threshold
                .or(self.similarity_threshold)
                .unwrap_or(d.similarity_threshold),
            max_gap_ms: max_gap_ms.or(self.max_gap_ms).unwrap_or(d.max_gap_ms),
        }
    }
}
