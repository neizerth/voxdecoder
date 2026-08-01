//! DOCX (OOXML) and best-effort legacy `.doc` via `textutil` on macOS.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::ZipArchive;

pub fn extract(path: &Path) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut zip =
        ZipArchive::new(file).map_err(|e| format!("{}: not a docx zip ({e})", path.display()))?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|e| format!("{}: missing word/document.xml ({e})", path.display()))?
        .read_to_string(&mut xml)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(strip_xml_text(&xml))
}

/// Legacy Word `.doc`: use macOS `textutil` when available.
pub fn extract_legacy_doc(path: &Path) -> Result<String, String> {
    let out = Command::new("textutil")
        .args(["-convert", "txt", "-stdout"])
        .arg(path)
        .output()
        .map_err(|e| {
            format!(
                "{}: legacy .doc requires textutil (macOS) or convert to .docx ({e})",
                path.display()
            )
        })?;
    if !out.status.success() {
        return Err(format!(
            "{}: textutil failed: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn strip_xml_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = String::new();
    let mut in_t = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"w:t" => in_t = true,
            Ok(Event::End(e)) if e.name().as_ref() == b"w:t" => in_t = false,
            Ok(Event::Text(t)) if in_t => {
                if let Ok(s) = t.unescape() {
                    if !out.is_empty() && !out.ends_with([' ', '\n']) {
                        out.push(' ');
                    }
                    out.push_str(&s);
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"w:p" || e.name().as_ref() == b"w:br" => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}
