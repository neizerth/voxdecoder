//! Artifact id wiring.

use std::path::PathBuf;

use vd_pipeline::{resolve_job, Capability, Job, JobInput, Step};

fn job_with_steps(steps: Vec<Step>) -> Job {
    Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/tmp")),
        input: JobInput {
            audio: Some(PathBuf::from("a.ogg")),
        },
        context: Default::default(),
        output: Default::default(),
        continue_on_error: false,
        steps,
    }
}

#[test]
fn unknown_artifact_id_errors() {
    let job = job_with_steps(vec![Step {
        r#use: Capability::FixCasing,
        id: None,
        name: None,
        input: Some("missing".into()),
        output: None,
        skip: false,
        options: Default::default(),
    }]);
    let err = resolve_job(job).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("unknown artifact id"));
}

#[test]
fn duplicate_id_errors() {
    let job = job_with_steps(vec![
        Step {
            r#use: Capability::Transcribe,
            id: Some("t".into()),
            name: None,
            input: None,
            output: None,
            skip: false,
            options: Default::default(),
        },
        Step {
            r#use: Capability::FixCasing,
            id: Some("t".into()),
            name: None,
            input: Some("t".into()),
            output: None,
            skip: false,
            options: Default::default(),
        },
    ]);
    let err = resolve_job(job).unwrap_err();
    assert!(err.to_string().contains("duplicate"));
}

#[test]
fn name_not_used_for_wiring() {
    let job = job_with_steps(vec![
        Step {
            r#use: Capability::Transcribe,
            id: Some("transcript".into()),
            name: Some("Initial".into()),
            input: None,
            output: None,
            skip: false,
            options: Default::default(),
        },
        Step {
            r#use: Capability::FixCasing,
            id: None,
            name: None,
            input: Some("Initial".into()),
            output: None,
            skip: false,
            options: Default::default(),
        },
    ]);
    let err = resolve_job(job).unwrap_err();
    assert!(err.to_string().contains("unknown artifact id"));
}
