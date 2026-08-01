//! Whisper / engine gate.

use super::fixture;
use vd_pipeline::{load_job_file, resolve_job};

#[test]
fn whisper_reserved_before_exec() {
    let job = load_job_file(&fixture("jobs/whisper.yaml")).unwrap();
    let err = resolve_job(job).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().to_lowercase().contains("whisper"));
}
