//! Output path resolution and writers.

pub mod formats;
pub mod writer;

use std::path::PathBuf;

use crate::config::resolve::OutputFormat;

pub use vd_output::{
    ensure_writable, resolve_output_path, segments_sidecar, stem_ext_file_name, OutputPathError,
    OutputPaths as ResolvedMain,
};

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

/// Resolve main transcript path (+ optional segments sidecar) for ASR output.
pub fn resolve_output_paths(req: OutputPathRequest) -> Result<OutputPaths, OutputPathError> {
    let default_file_name = stem_ext_file_name(&req.input, req.format.extension());
    let resolved = resolve_output_path(vd_output::OutputPathRequest {
        input: req.input,
        output: req.output,
        output_dir: req.output_dir,
        in_place: false,
        overwrite: req.overwrite,
        default_file_name,
    })?;

    let segments = if req.segments {
        let seg = segments_sidecar(&resolved.main);
        ensure_writable(&seg, req.overwrite)?;
        Some(seg)
    } else {
        None
    };

    Ok(OutputPaths {
        main: resolved.main,
        segments,
    })
}
