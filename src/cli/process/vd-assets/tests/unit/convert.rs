//! Convert pipeline.

use std::fs;
use std::io::Write;

use tempfile::TempDir;
use vd_assets::convert::{run, ConvertRequest, OcrMode};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn converts_docx_when_no_text_sources() {
    let dir = TempDir::new().unwrap();
    let docx = dir.path().join("spec.docx");
    write_minimal_docx(&docx, "AcmeCloud Kubernetes");
    let out = dir.path().join("out");

    let result = run(&ConvertRequest {
        inputs: vec![docx],
        output_dir: out.clone(),
        ocr: OcrMode::Off,
        force: true,
    })
    .unwrap();

    assert!(result.terms_path.exists());
    assert_eq!(
        result.terms_path.file_name().and_then(|n| n.to_str()),
        Some("terms.yml")
    );
    assert!(!result.converted.is_empty());
    let md = fs::read_to_string(&result.converted[0]).unwrap();
    assert!(md.contains("Kubernetes"));
    assert!(result
        .dictionary
        .forms
        .iter()
        .any(|f| f.contains("Kubernetes") || f.contains("AcmeCloud")));
}

#[test]
fn builds_from_markdown_without_office() {
    let dir = TempDir::new().unwrap();
    let md = dir.path().join("notes.md");
    fs::write(&md, "- `foo` → BarCorp\n").unwrap();
    let out = dir.path().join("out");

    let result = run(&ConvertRequest {
        inputs: vec![md],
        output_dir: out,
        ocr: OcrMode::Off,
        force: false,
    })
    .unwrap();

    assert!(result.converted.is_empty());
    assert!(result
        .dictionary
        .entries
        .iter()
        .any(|e| e.canonical == "BarCorp"));
}

fn write_minimal_docx(path: &std::path::Path, text: &str) {
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"></Types>"#).unwrap();
    zip.start_file("word/document.xml", opts).unwrap();
    let xml = format!(
        r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:body></w:document>"#
    );
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
}
