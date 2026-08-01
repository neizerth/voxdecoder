//! PDF text layer via `pdf-extract`.

use std::fs;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    pdf_extract::extract_text_from_mem(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}
