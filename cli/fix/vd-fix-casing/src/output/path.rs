//! `-o` XOR `-d` XOR `--in-place`, `.fixed.`, overwrite checks.

use std::path::{Path, PathBuf};

use crate::types::ArtifactType;

#[derive(Debug, Clone)]
pub struct OutputPathRequest {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub artifact_type: ArtifactType,
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPaths {
    pub main: PathBuf,
    pub in_place: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum OutputPathError {
    #[error("--output, --output-dir, and --in-place are mutually exclusive")]
    ConflictingTargets,
    #[error("output already exists: {0}")]
    AlreadyExists(PathBuf),
}

impl OutputPathError {
    pub fn exit_code(&self) -> u8 {
        2
    }
}

pub fn resolve_output_path(req: OutputPathRequest) -> Result<OutputPaths, OutputPathError> {
    let set = u8::from(req.output.is_some())
        + u8::from(req.output_dir.is_some())
        + u8::from(req.in_place);
    if set > 1 {
        return Err(OutputPathError::ConflictingTargets);
    }

    let (main, in_place) = if req.in_place {
        (req.input.clone(), true)
    } else if let Some(o) = req.output {
        (o, false)
    } else if let Some(dir) = req.output_dir {
        let name = fixed_file_name(&req.input, req.artifact_type);
        (dir.join(name), false)
    } else {
        let parent = req
            .input
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let name = fixed_file_name(&req.input, req.artifact_type);
        (parent.join(name), false)
    };

    if !req.overwrite && !in_place && main.exists() {
        return Err(OutputPathError::AlreadyExists(main));
    }

    Ok(OutputPaths { main, in_place })
}

fn fixed_file_name(input: &Path, artifact_type: ArtifactType) -> String {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    format!("{stem}.fixed.{}", artifact_type.extension())
}
