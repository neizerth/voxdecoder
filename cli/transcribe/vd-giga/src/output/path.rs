//! `-o` XOR `-d`, default next to input, segments sidecar, overwrite checks.

use std::path::{Path, PathBuf};

use crate::config::resolve::OutputFormat;

#[derive(Debug, Clone)]
pub struct OutputPathRequest {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub format: OutputFormat,
    pub segments: bool,
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPaths {
    pub main: PathBuf,
    pub segments: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum OutputPathError {
    #[error("--output and --output-dir are mutually exclusive")]
    OutputAndDir,
    #[error("output already exists: {0}")]
    AlreadyExists(PathBuf),
}

impl OutputPathError {
    pub fn exit_code(&self) -> u8 {
        2
    }
}

pub fn resolve_output_paths(req: OutputPathRequest) -> Result<OutputPaths, OutputPathError> {
    if req.output.is_some() && req.output_dir.is_some() {
        return Err(OutputPathError::OutputAndDir);
    }

    let main = if let Some(o) = req.output {
        o
    } else if let Some(dir) = req.output_dir {
        let stem = stem_of(&req.input);
        dir.join(format!("{stem}.{}", req.format.extension()))
    } else {
        let stem = stem_of(&req.input);
        let parent = req
            .input
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        parent.join(format!("{stem}.{}", req.format.extension()))
    };

    let segments = if req.segments {
        Some(segments_path_for(&main))
    } else {
        None
    };

    if !req.overwrite {
        if main.exists() {
            return Err(OutputPathError::AlreadyExists(main));
        }
        if let Some(ref seg) = segments {
            if seg.exists() {
                return Err(OutputPathError::AlreadyExists(seg.clone()));
            }
        }
    }

    Ok(OutputPaths { main, segments })
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
}

fn segments_path_for(main: &Path) -> PathBuf {
    let parent = main.parent().unwrap_or_else(|| Path::new("."));
    let stem = stem_of(main);
    parent.join(format!("{stem}.segments.json"))
}
