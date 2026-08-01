//! Default Job from CLI flags.

use std::path::PathBuf;

use super::fixture;
use vd_pipeline::{default_job, load_job_file, Capability, DefaultJobArgs, TranscribeEngine};

#[test]
fn default_matches_fixture() {
    let expected = load_job_file(&fixture("jobs/default.yaml")).unwrap();
    let got = default_job(&DefaultJobArgs {
        audio: PathBuf::from("meeting.ogg"),
        engine: TranscribeEngine::Gigaam,
        model: None,
        docs: None,
        output_dir: None,
        working_dir: None,
        continue_on_error: false,
        overwrite: false,
    });
    assert_eq!(got, expected);
}

#[test]
fn docs_inserts_prepare_context() {
    let job = default_job(&DefaultJobArgs {
        audio: PathBuf::from("meeting.ogg"),
        engine: TranscribeEngine::Gigaam,
        model: None,
        docs: Some(PathBuf::from("./docs")),
        output_dir: None,
        working_dir: None,
        continue_on_error: false,
        overwrite: false,
    });
    assert!(job
        .steps
        .iter()
        .any(|s| s.r#use == Capability::PrepareContext));
    assert_eq!(job.context.docs, Some(PathBuf::from("./docs")));
}
