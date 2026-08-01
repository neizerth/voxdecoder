//! Spreadsheet text via `calamine`.

use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};

pub fn extract(path: &Path) -> Result<String, String> {
    let mut workbook = open_workbook_auto(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = String::new();
    let sheets = workbook.sheet_names().clone();
    for name in sheets {
        if let Ok(range) = workbook.worksheet_range(&name) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&name);
            out.push('\n');
            for row in range.rows() {
                let line: Vec<String> = row
                    .iter()
                    .map(|c| match c {
                        Data::Empty => String::new(),
                        other => other.to_string(),
                    })
                    .collect();
                if line.iter().any(|s| !s.is_empty()) {
                    out.push_str(&line.join("\t"));
                    out.push('\n');
                }
            }
        }
    }
    Ok(out)
}
