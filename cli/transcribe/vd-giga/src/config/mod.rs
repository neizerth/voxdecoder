//! Config load / save / merge.

pub mod file;
pub mod resolve;

use resolve::{Device, OutputFormat};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub model: Option<String>,
    pub device: Option<Device>,
    pub fp16_encoder: Option<bool>,
    pub flash: Option<bool>,
    pub download_root: Option<String>,
    pub word_timestamps: Option<bool>,
    pub format: Option<OutputFormat>,
}

impl FileConfig {
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "model" => self.model.clone(),
            "device" => self.device.map(|d| d.as_str().to_string()),
            "fp16_encoder" => self.fp16_encoder.map(bool_on_off),
            "flash" => self.flash.map(bool_on_off),
            "download_root" => self.download_root.clone().or_else(|| Some(String::new())),
            "word_timestamps" => self.word_timestamps.map(bool_on_off),
            "format" => self.format.map(|f| f.as_str().to_string()),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "model" => self.model = Some(value.to_string()),
            "device" => {
                self.device =
                    Some(Device::parse(value).ok_or_else(|| {
                        format!(
                            "invalid device '{value}' (expected {})",
                            Device::allowed().join("|")
                        )
                    })?);
            }
            "fp16_encoder" => self.fp16_encoder = Some(parse_on_off(value)?),
            "flash" => {
                if !crate::platform::FLASH_SUPPORTED {
                    return Err("flash is not available on this platform".into());
                }
                self.flash = Some(parse_on_off(value)?);
            }
            "download_root" => {
                self.download_root = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "word_timestamps" => self.word_timestamps = Some(parse_on_off(value)?),
            "format" => {
                self.format = Some(OutputFormat::parse(value).ok_or_else(|| {
                    format!("invalid format '{value}' (expected txt|json|srt|vtt)")
                })?);
            }
            _ => return Err(format!("unknown config key '{key}'")),
        }
        Ok(())
    }

    pub fn list_lines(&self) -> Vec<String> {
        let defaults = resolve::defaults();
        vec![
            format!(
                "model = {}",
                self.model.as_deref().unwrap_or(&defaults.model)
            ),
            format!(
                "device = {}",
                self.device.unwrap_or(defaults.device).as_str()
            ),
            format!(
                "fp16_encoder = {}",
                bool_on_off(self.fp16_encoder.unwrap_or(defaults.fp16_encoder))
            ),
    format!(
                "flash = {}",
                if crate::platform::FLASH_SUPPORTED {
                    bool_on_off(self.flash.unwrap_or(defaults.flash))
                } else {
                    "off".into()
                }
            ),
            format!(
                "download_root = {}",
                self.download_root.as_deref().unwrap_or("")
            ),
            format!(
                "word_timestamps = {}",
                bool_on_off(self.word_timestamps.unwrap_or(defaults.word_timestamps))
            ),
            format!(
                "format = {}",
                self.format.unwrap_or(defaults.format).as_str()
            ),
        ]
    }
}

fn bool_on_off(v: bool) -> String {
    if v { "on" } else { "off" }.to_string()
}

fn parse_on_off(value: &str) -> Result<bool, String> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("expected on|off, got '{value}'")),
    }
}
