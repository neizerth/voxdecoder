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
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: steps.into_iter().map(Into::into).collect(),
    }
}

#[test]
fn unknown_artifact_id_errors() {
    let job = job_with_steps(vec![Step {
        input: Some("missing".into()),
        ..Step::new(Capability::FixCasing)
    }]);
    let err = resolve_job(job).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("unknown artifact id"));
}

#[test]
fn duplicate_id_errors() {
    let job = job_with_steps(vec![
        Step {
            id: Some("t".into()),
            ..Step::new(Capability::Transcribe)
        },
        Step {
            id: Some("t".into()),
            input: Some("t".into()),
            ..Step::new(Capability::FixCasing)
        },
    ]);
    let err = resolve_job(job).unwrap_err();
    assert!(err.to_string().contains("duplicate"));
}

#[test]
fn name_not_used_for_wiring() {
    let job = job_with_steps(vec![
        Step {
            id: Some("transcript".into()),
            name: Some("Initial".into()),
            ..Step::new(Capability::Transcribe)
        },
        Step {
            input: Some("Initial".into()),
            ..Step::new(Capability::FixCasing)
        },
    ]);
    let err = resolve_job(job).unwrap_err();
    assert!(err.to_string().contains("unknown artifact id"));
}

#[test]
fn inputs_list_sugar() {
    let job = job_with_steps(vec![
        Step {
            id: Some("transcript".into()),
            ..Step::new(Capability::Transcribe)
        },
        Step {
            inputs: vec!["transcript".into()],
            ..Step::new(Capability::FixCasing)
        },
    ]);
    assert!(resolve_job(job).is_ok());
}
