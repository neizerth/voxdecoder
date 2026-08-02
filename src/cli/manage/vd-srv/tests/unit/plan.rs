//! Runtime planning tests.

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vd_meeting::InputRole;
use vd_srv::plan::{
    plan_audio, plan_meeting, AudioRequest, InputSource, MeetingInput, MeetingPlanRequest,
};
use vd_srv::store::{ArtifactEntry, JobStore, Priority, RestartPolicy};
use vd_pipeline::{default_job, DefaultJobArgs, TranscribeEngine};

#[test]
fn resolves_path_and_file_uri_inputs() {
    let path = InputSource {
        path: Some("/tmp/audio.wav".into()),
        uri: None,
        artifact: None,
        blob: None,
    };
    assert_eq!(
        path.resolve(Path::new("/tmp"), None).unwrap(),
        Path::new("/tmp/audio.wav")
    );

    let uri = InputSource {
        path: None,
        uri: Some("file:///tmp/audio.wav".into()),
        artifact: None,
        blob: None,
    };
    assert_eq!(
        uri.resolve(Path::new("/tmp"), None).unwrap(),
        Path::new("/tmp/audio.wav")
    );
}

#[test]
fn resolves_artifact_input() {
    let dir = TempDir::new().unwrap();
    let data = dir.path().join("data");
    fs::create_dir_all(&data).unwrap();
    let store = JobStore::open(&data).unwrap();
    let job = default_job(&DefaultJobArgs {
        audio: Path::new("/tmp/in.wav").to_path_buf(),
        engine: TranscribeEngine::Gigaam,
        model: None,
        device: None,
        flash: false,
        docs: None,
        output_dir: None,
        working_dir: None,
        continue_on_error: false,
        overwrite: false,
    });
    let record = store
        .create(job, Priority::default(), RestartPolicy::default())
        .unwrap();
    let art_path = dir.path().join("prior.wav");
    fs::write(&art_path, b"wav").unwrap();
    store
        .write_artifacts(
            &record.id,
            &[ArtifactEntry {
                id: "prior-audio".into(),
                path: art_path.clone(),
                kind: Some("audio".into()),
                producer: None,
            }],
        )
        .unwrap();

    let by_id = InputSource {
        path: None,
        uri: None,
        artifact: Some("prior-audio".into()),
        blob: None,
    };
    assert_eq!(
        by_id.resolve(&data, Some(&store)).unwrap(),
        art_path
    );

    let scoped = InputSource {
        path: None,
        uri: None,
        artifact: Some(format!("{}:prior-audio", record.id)),
        blob: None,
    };
    assert_eq!(scoped.resolve(&data, Some(&store)).unwrap(), art_path);
}

#[test]
fn audio_plan_has_job_shape() {
    let request = AudioRequest {
        audio: InputSource {
            path: Some("/tmp/audio.wav".into()),
            uri: None,
            artifact: None,
            blob: None,
        },
        engine: None,
        model: None,
        device: None,
        flash: false,
        docs: None,
        output_dir: None,
        working_dir: None,
        continue_on_error: false,
        overwrite: false,
    };
    let job = plan_audio(&request, Path::new("/tmp"), None).unwrap();
    assert_eq!(
        job.input.audio.as_deref(),
        Some(Path::new("/tmp/audio.wav"))
    );
    assert!(!job.steps.is_empty());
}

#[test]
fn meeting_plan_from_audio_convenience() {
    let request = MeetingPlanRequest {
        working_dir: Some(Path::new("/work").to_path_buf()),
        inputs: Vec::new(),
        meeting: Default::default(),
        output: Default::default(),
        audio: Some(InputSource {
            path: Some("/work/meeting.wav".into()),
            uri: None,
            artifact: None,
            blob: None,
        }),
        options: Default::default(),
        engine: None,
        model: None,
        document: None,
        meeting_yaml: None,
    };
    let job = plan_meeting(&request, Path::new("/tmp"), None).unwrap();
    assert!(!job.steps.is_empty());
    assert_eq!(job.working_dir.as_deref(), Some(Path::new("/work")));
}

#[test]
fn meeting_plan_from_inputs() {
    let request = MeetingPlanRequest {
        working_dir: Some(Path::new("/work").to_path_buf()),
        inputs: vec![MeetingInput {
            role: InputRole::Room,
            path: Some("/work/room.wav".into()),
            uri: None,
            artifact: None,
            blob: None,
            participant: None,
            purposes: Vec::new(),
        }],
        meeting: Default::default(),
        output: Default::default(),
        audio: None,
        options: Default::default(),
        engine: None,
        model: None,
        document: None,
        meeting_yaml: None,
    };
    let job = plan_meeting(&request, Path::new("/tmp"), None).unwrap();
    assert!(!job.steps.is_empty());
}
