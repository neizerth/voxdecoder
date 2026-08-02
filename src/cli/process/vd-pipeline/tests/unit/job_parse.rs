//! Job file parse.

use super::fixture;
use vd_pipeline::{load_job_file, Capability};

#[test]
fn yaml_and_json_round_trip_default() {
    let from_yaml = load_job_file(&fixture("jobs/default.yaml")).unwrap();
    let from_json = load_job_file(&fixture("jobs/default.json")).unwrap();
    assert_eq!(from_yaml, from_json);
    assert_eq!(from_yaml.version, 1);
    assert_eq!(from_yaml.leaf_count(), 4);
    assert_eq!(from_yaml.leaf_steps()[0].r#use, Capability::Transcribe);
    assert_eq!(from_yaml.leaf_steps()[0].id.as_deref(), Some("transcript"));
    assert!(from_yaml.leaf_steps()[0].options.contains_key("engine"));
}

#[test]
fn full_job_loads() {
    let job = load_job_file(&fixture("jobs/full.yaml")).unwrap();
    assert_eq!(job.name.as_deref(), Some("meeting cleanup"));
    assert!(job.context.docs.is_some());
    assert_eq!(job.leaf_count(), 5);
    assert_eq!(job.leaf_steps()[1].r#use, Capability::PrepareContext);
}

#[test]
fn unknown_use_errors() {
    let err = load_job_file(&fixture("jobs/bad_use.yaml")).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn empty_steps_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.yaml");
    std::fs::write(&path, "version: 1\nsteps: []\n").unwrap();
    let err = load_job_file(&path).unwrap_err();
    assert!(err.to_string().contains("no steps"));
}

#[test]
fn diarize_nested_backend_options() {
    use vd_pipeline::ArgValue;

    let job = load_job_file(&fixture("jobs/diarize.yaml")).unwrap();
    assert_eq!(job.leaf_count(), 1);
    assert_eq!(job.leaf_steps()[0].r#use, Capability::Diarize);
    let backend = job.leaf_steps()[0]
        .options
        .get("backend")
        .and_then(ArgValue::as_map)
        .expect("nested backend map");
    assert_eq!(
        backend.get("provider").and_then(ArgValue::as_string).as_deref(),
        Some("stub")
    );
    assert_eq!(
        backend.get("model").and_then(ArgValue::as_string).as_deref(),
        Some("deterministic-v1")
    );
}
