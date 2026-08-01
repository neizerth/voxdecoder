//! Plain UTF-8 files.

use std::fs;
use std::path::Path;

pub fn read_utf8(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}
