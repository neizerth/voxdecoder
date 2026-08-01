//! Optional OCR via local `tesseract` CLI.

use std::path::Path;
use std::process::Command;

/// Run `tesseract <path> stdout -l rus+eng`. Returns Err if binary missing / failed.
pub fn run_tesseract(path: &Path) -> Result<String, String> {
    let out = Command::new("tesseract")
        .arg(path)
        .arg("stdout")
        .args(["-l", "rus+eng"])
        .output()
        .map_err(|e| format!("tesseract not available: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "tesseract failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
