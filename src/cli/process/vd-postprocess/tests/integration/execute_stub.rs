//! Stub runner end-to-end via library.

use std::collections::BTreeMap;
use std::fs;

use tempfile::TempDir;
use vd_postprocess::{
    execute, plan, ArtifactBinding, PostprocessRequest, RunnerSpec,
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
            artifact: Some("transcript".into()),
            format: None,
        },
    );
    let req = PostprocessRequest {
        inputs,
        recipes: vec![recipe],
        runner: RunnerSpec::with_type("stub"),
        variables: BTreeMap::new(),
        output_dir: Some(dir.path().to_path_buf()),
        overwrite: true,
    };

    let planned = plan(&req).unwrap();
    assert_eq!(planned.nodes.len(), 1);
    assert_eq!(planned.nodes[0].outputs.len(), 2);
    assert!(planned.nodes[0].parallel);
    assert_eq!(planned.steps.len(), 1);

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
fn graph_two_nodes_with_from() {
    let dir = TempDir::new().unwrap();
    let transcript = dir.path().join("t.txt");
    fs::write(&transcript, "raw").unwrap();
    let recipe = dir.path().join("r.yaml");
    fs::write(
        &recipe,
        r#"
version: 1
id: chain
inputs:
  transcript:
    required: true
runner:
  type: stub
outputs:
  report:
    artifact: report
    type: markdown
graph:
  - id: extract
    prompt: |
      EXTRACT {{ transcript }}
    outputs:
      entities:
        artifact: entities
        type: json
  - id: render
    needs: [extract]
    prompt: |
      RENDER {{ draft }}
    inputs:
      draft:
        from: extract.entities
    outputs:
      report:
        artifact: report
        type: markdown
"#,
    )
    .unwrap();

    let mut inputs = BTreeMap::new();
    inputs.insert(
        "transcript".into(),
        ArtifactBinding {
            path: transcript,
            artifact: None,
            format: None,
        },
    );
    let req = PostprocessRequest {
        inputs,
        recipes: vec![recipe],
        runner: RunnerSpec::default(),
        variables: BTreeMap::new(),
        output_dir: Some(dir.path().to_path_buf()),
        overwrite: true,
    };
    let planned = plan(&req).unwrap();
    assert_eq!(planned.nodes.len(), 2);
    assert!(planned.nodes[0].parallel);
    assert!(!planned.nodes[1].parallel);
    assert_eq!(planned.nodes[1].needs, vec!["extract".to_string()]);

    execute(&req).unwrap();
    assert!(dir.path().join("entities.json").exists());
    assert!(dir.path().join("report.md").exists());
    let report = fs::read_to_string(dir.path().join("report.md")).unwrap();
    assert!(report.contains("RENDER"));
}

#[test]
fn map_outputs_and_runner_alias() {
    let dir = TempDir::new().unwrap();
    let transcript = dir.path().join("t.txt");
    fs::write(&transcript, "x").unwrap();
    let recipe = dir.path().join("r.yaml");
    fs::write(
        &recipe,
        r#"
version: 1
provider:
  type: stub
  model: m1
inputs:
  transcript:
    required: true
outputs:
  summary:
    artifact: summary
    type: markdown
    path: reports/summary.md
prompt: |
  {{ transcript }}
"#,
    )
    .unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "transcript".into(),
        ArtifactBinding {
            path: transcript,
            artifact: None,
            format: None,
        },
    );
    let req = PostprocessRequest {
        inputs,
        recipes: vec![recipe],
        runner: RunnerSpec::default(),
        variables: BTreeMap::new(),
        output_dir: Some(dir.path().to_path_buf()),
        overwrite: true,
    };
    let planned = plan(&req).unwrap();
    assert_eq!(planned.nodes[0].runner.r#type, "stub");
    assert_eq!(planned.nodes[0].runner.model.as_deref(), Some("m1"));
    execute(&req).unwrap();
    assert!(dir.path().join("reports/summary.md").exists());
}

#[test]
fn no_recipes_errors() {
    let err = plan(&PostprocessRequest {
        inputs: BTreeMap::from([(
            "a".into(),
            ArtifactBinding {
                path: "/tmp".into(),
                artifact: None,
                format: None,
            },
        )]),
        recipes: vec![],
        runner: RunnerSpec::default(),
        variables: BTreeMap::new(),
        output_dir: None,
        overwrite: false,
    })
    .unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("no recipes"));
}
