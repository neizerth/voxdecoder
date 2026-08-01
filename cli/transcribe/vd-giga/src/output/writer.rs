//! Write main transcript and optional segments sidecar.

use std::fs;
use std::io;
use std::path::Path;

use crate::config::resolve::OutputFormat;
use crate::gigaam::model::Transcript;
use crate::output::formats::{self, JsonResult};

pub fn write_outputs(
    main: &Path,
    segments: Option<&Path>,
    format: OutputFormat,
    transcript: &Transcript,
) -> io::Result<()> {
    if let Some(parent) = main.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(main, formats::render(format, transcript))?;

    if let Some(seg_path) = segments {
        if let Some(parent) = seg_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = JsonResult {
            text: &transcript.text,
            segments: &transcript.segments,
            words: transcript.words.as_deref(),
        };
        fs::write(seg_path, serde_json::to_string_pretty(&body)?)?;
    }
    Ok(())
}
