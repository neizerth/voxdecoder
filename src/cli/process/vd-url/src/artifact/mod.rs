//! Artifact writers for import outputs.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::import::ImportError;

pub fn write_metadata(dir: &Path, value: &impl Serialize) -> Result<PathBuf, ImportError> {
    fs::create_dir_all(dir).map_err(|e| ImportError::Io(e.to_string()))?;
    let path = dir.join("metadata.yaml");
    let body = serde_yaml::to_string(value).map_err(|e| ImportError::Io(e.to_string()))?;
    fs::write(&path, body).map_err(|e| ImportError::Io(e.to_string()))?;
    Ok(path)
}
