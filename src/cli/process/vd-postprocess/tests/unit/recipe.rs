//! Recipe load / validate.

use std::fs;

use tempfile::TempDir;
use vd_postprocess::postprocess::recipe::load_recipe;

#[test]
fn load_valid_recipe() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("summary.yaml");
    fs::write(
        &path,
        r#"
version: 1
id: summary
inputs:
  transcript:
    required: true
variables:
  audience: Engineering
outputs:
  - id: summary
    path: summary.md
    mime: text/markdown
prompt: |
  Hello {{ audience }}
  {{ transcript }}
"#,
    )
    .unwrap();
    let doc = load_recipe(&path).unwrap();
    assert_eq!(doc.id.as_deref(), Some("summary"));
    assert_eq!(doc.outputs.len(), 1);
}

#[test]
fn reject_no_outputs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.yaml");
    fs::write(
        &path,
        "version: 1\nprompt: hi\noutputs: []\n",
    )
    .unwrap();
    assert!(load_recipe(&path).is_err());
}
