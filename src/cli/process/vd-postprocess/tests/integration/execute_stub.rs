//! Stub provider end-to-end via library.

use std::collections::BTreeMap;
use std::fs;

use tempfile::TempDir;
use vd_postprocess::{
    execute, plan, ArtifactBinding, ExecutionProviderSpec, PostprocessRequest,
};

#[test]
fn stub_writes_outputs() {
    let dir = TempDir::new().unwrap();
    let transcript = dir.path().join("t.txt");
    fs::write(&transcript, "hello world").unwrap();
    let recipe = dir.path().join("r.yaml");
    fs::write(
        &recipe,
        r#"
version: 1
id: summary
inputs:
  transcript:
    required: true
outputs:
  - id: summary
    path: summary.md
  - id: tasks
    path: tasks.json
    format: json
prompt: |
  Body: {{ transcript }}
"#,
    )
    .unwrap();

    let mut inputs = BTreeMap::new();
    inputs.insert(
        "transcript".into(),
        ArtifactBinding {
            path: transcript,
        },
    );
    let req = PostprocessRequest {
        inputs,
        recipes: vec![recipe],
        provider: ExecutionProviderSpec {
            r#type: "stub".into(),
            ..Default::default()
        },
        variables: BTreeMap::new(),
        output_dir: Some(dir.path().to_path_buf()),
        overwrite: true,
    };

    let planned = plan(&req).unwrap();
    assert_eq!(planned.steps.len(), 1);
    assert_eq!(planned.steps[0].outputs.len(), 2);

    let result = execute(&req).unwrap();
    assert_eq!(result.results[0].outputs.len(), 2);
    let summary = dir.path().join("summary.md");
    let tasks = dir.path().join("tasks.json");
    assert!(summary.exists());
    assert!(tasks.exists());
    let text = fs::read_to_string(summary).unwrap();
    assert!(text.contains("hello world"));
}

#[test]
fn no_recipes_errors() {
    let err = plan(&PostprocessRequest {
        inputs: BTreeMap::from([(
            "a".into(),
            ArtifactBinding {
                path: "/tmp".into(),
            },
        )]),
        recipes: vec![],
        provider: ExecutionProviderSpec::default(),
        variables: BTreeMap::new(),
        output_dir: None,
        overwrite: false,
    })
    .unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("no recipes"));
}
