//! Stub provider via library.

use std::fs;

use tempfile::TempDir;
use vd_preprocess::{
    execute, plan, request_from_raw, FilterSpec, PreprocessRequest, RawFilter,
};

#[test]
fn stub_chain_copies() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("in.wav");
    fs::write(&input, b"RIFF....WAVEfmt ").unwrap();
    let chain = dir.path().join("c.yaml");
    fs::write(
        &chain,
        r#"
provider: stub
filters:
  - type: normalize
  - type: resample
    rate: 16000
"#,
    )
    .unwrap();

    let raw = vec![
        RawFilter {
            provider: None,
            operation: None,
            r#type: Some("normalize".into()),
            params: Default::default(),
        },
        RawFilter {
            provider: None,
            operation: None,
            r#type: Some("resample".into()),
            params: [("rate".into(), serde_json::json!(16000))]
                .into_iter()
                .collect(),
        },
    ];
    let out = dir.path().join("out.wav");
    let req = request_from_raw(
        input.clone(),
        raw,
        "stub",
        Some(out.clone()),
        None,
        true,
    )
    .unwrap();
    let planned = plan(&req).unwrap();
    assert_eq!(planned.steps.len(), 2);
    assert_eq!(planned.steps[0].operation, "normalize");
    let tmp = planned.steps[0].output.file_name().unwrap().to_string_lossy();
    assert!(
        tmp.starts_with(".vd-preprocess-") && tmp.contains("-0-normalize.tmp"),
        "expected unique tagged temp, got {tmp}"
    );

    let result = execute(&req).unwrap();
    assert_eq!(result.output.path, out);
    assert!(out.exists());
    assert_eq!(fs::read(&out).unwrap(), fs::read(&input).unwrap());
}

#[test]
fn no_filters_errors() {
    let err = plan(&PreprocessRequest {
        input: "/tmp".into(),
        filters: Vec::<FilterSpec>::new(),
        provider: None,
        output: None,
        output_dir: None,
        overwrite: false,
    })
    .unwrap_err();
    assert_eq!(err.exit_code(), 2);
}
