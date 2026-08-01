//! Load / inference options for `GigaModel`.

use std::path::PathBuf;

use crate::config::resolve::Device;

#[derive(Debug, Clone)]
pub struct GigaLoadOptions {
    pub model: String,
    pub device: Device,
    pub fp16_encoder: bool,
    pub flash: bool,
    pub download_root: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct TranscribeOptions {
    pub word_timestamps: bool,
}
