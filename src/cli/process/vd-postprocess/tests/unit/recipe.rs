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
fn load_graph_map_outputs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("g.yaml");
    fs::write(
        &path,
        r#"
version: 1
id: g
runner:
  type: stub
inputs:
  transcript:
    required: true
secrets:
  token: env:VD_TEST_TOKEN
outputs:
  summary:
    artifact: summary
    type: markdown
graph:
  - id: main
    prompt: hi {{ transcript }}
"#,
    )
    .unwrap();
    let doc = load_recipe(&path).unwrap();
    assert_eq!(doc.graph.len(), 1);
    assert_eq!(doc.outputs[0].artifact, "summary");
    assert!(doc.secrets.contains_key("token"));
}

#[test]
fn reject_no_outputs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.yaml");
    fs::write(&path, "version: 1\nprompt: hi\noutputs: []\n").unwrap();
    assert!(load_recipe(&path).is_err());
}

#[test]
fn reject_plain_secret() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.yaml");
    fs::write(
        &path,
        "version: 1\nprompt: hi\noutputs:\n  - id: o\n    path: o.md\nsecrets:\n  k: plaintext\n",
    )
    .unwrap();
    assert!(load_recipe(&path).is_err());
}
