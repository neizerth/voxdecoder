//! Output path resolution for transcript / fix CLIs (filesystem only).
//!
//! Callers supply the **default file name** scheme (`meeting.fixed.txt`,
//! `meeting.txt`, …). This crate owns `-o` / `-d` / `--in-place` / `--overwrite`.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct OutputPathRequest {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub overwrite: bool,
    /// Used when neither `-o` nor `--in-place` (default next to input, or under `-d`).
    pub default_file_name: String,
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
        (dir.join(&req.default_file_name), false)
    } else {
        let parent = req
            .input
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        (parent.join(&req.default_file_name), false)
    };

    if !req.overwrite && !in_place {
        ensure_writable(&main, false)?;
    }

    Ok(OutputPaths { main, in_place })
}

/// Fail if `path` exists and `overwrite` is false.
pub fn ensure_writable(path: &Path, overwrite: bool) -> Result<(), OutputPathError> {
    if !overwrite && path.exists() {
        return Err(OutputPathError::AlreadyExists(path.to_path_buf()));
    }
    Ok(())
}

pub fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
}

/// Fix-CLI default: `{stem}.fixed.{ext}`.
pub fn fixed_file_name(input: &Path, extension: &str) -> String {
    format!("{}.fixed.{extension}", file_stem(input))
}

/// Transcription default: `{stem}.{ext}`.
pub fn stem_ext_file_name(input: &Path, extension: &str) -> String {
    format!("{}.{extension}", file_stem(input))
}

/// Sidecar next to `main`: `{stem}.segments.json`.
pub fn segments_sidecar(main: &Path) -> PathBuf {
    let parent = main.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.segments.json", file_stem(main)))
}
