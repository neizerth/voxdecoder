//! working_dir + relative path resolve.

use std::path::PathBuf;

use vd_pipeline::{resolve_job, Capability, Job, JobContext, JobInput, Step};

#[test]
fn relative_paths_join_working_dir() {
    let job = Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("meeting.ogg")),
        },
        context: JobContext {
            docs: Some(PathBuf::from("docs")),
            assets: None,
        },
        output: Default::default(),
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![
            Step {
                id: Some("transcript".into()),
                ..Step::new(Capability::Transcribe)
            },
            Step::new(Capability::PrepareContext),
        ],
    };
    let resolved = resolve_job(job).unwrap();
    assert_eq!(resolved.working_dir, PathBuf::from("/work"));
    assert_eq!(
        resolved.steps[0].input.as_deref(),
        Some(PathBuf::from("/work/meeting.ogg").as_path())
    );
    assert_eq!(
        resolved.steps[1].input.as_deref(),
        Some(PathBuf::from("/work/docs").as_path())
    );
}

#[test]
fn diarize_resolves_from_audio() {
    let job = Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("meeting.wav")),
        },
        context: Default::default(),
        output: Default::default(),
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![Step::new(Capability::Diarize)],
    };
    let resolved = resolve_job(job).unwrap();
    assert_eq!(
        resolved.steps[0].input.as_deref(),
        Some(PathBuf::from("/work/meeting.wav").as_path())
    );
}
