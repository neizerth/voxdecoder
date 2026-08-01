//! Detect `ArtifactType` from path extension.

use std::path::Path;

use crate::types::ArtifactType;

pub fn detect_type(path: &Path) -> Option<ArtifactType> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "txt" => Some(ArtifactType::Txt),
        "json" => Some(ArtifactType::Json),
        "jsonl" => Some(ArtifactType::Jsonl),
        "srt" => Some(ArtifactType::Srt),
        "vtt" => Some(ArtifactType::Vtt),
        "md" | "markdown" => Some(ArtifactType::Md),
        _ => None,
    }
}
