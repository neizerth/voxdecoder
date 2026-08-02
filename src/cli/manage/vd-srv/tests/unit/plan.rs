//! Runtime planning tests (resolve → plan).

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vd_meeting::InputRole;
use vd_pipeline::{default_job, Capability, DefaultJobArgs, TranscribeEngine};
use vd_srv::plan::{
    plan_audio, plan_meeting, AudioRequest, InputSource, MeetingInput, MeetingPlanRequest,
};
use vd_srv::store::{ArtifactEntry, JobStore, Priority, RestartPolicy};

fn path_src(path: &str) -> InputSource {
    InputSource {
        path: Some(path.into()),
        ..Default::default()
    }
}

#[test]
fn resolves_path_and_file_uri_inputs() {
    let dir = TempDir::new().unwrap();
    let path = path_src("/tmp/audio.wav");
    let job = plan_audio(
        &AudioRequest {
            audio: path,
            engine: None,
            model: None,
            device: None,
            flash: false,
            speed: None,
            subtitles: None,
            provider: None,
            docs: None,
            output_dir: None,
            working_dir: None,
            continue_on_error: false,
            overwrite: false,
        },
        dir.path(),
        None,
    )
    .unwrap();
    assert_eq!(
        job.input.audio.as_deref(),
        Some(Path::new("/tmp/audio.wav"))
    );

    let uri = InputSource {
        uri: Some("file:///tmp/audio.wav".into()),
        ..Default::default()
    };
    let job = plan_audio(
        &AudioRequest {
            audio: uri,
            engine: None,
            model: None,
            device: None,
            flash: false,
            speed: None,
            subtitles: None,
            provider: None,
            docs: None,
            output_dir: None,
            working_dir: None,
            continue_on_error: false,
            overwrite: false,
        },
        dir.path(),
        None,
    )
    .unwrap();
    assert_eq!(
        job.input.audio.as_deref(),
        Some(Path::new("/tmp/audio.wav"))
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
        speed: None,
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
        artifact: Some("prior-audio".into()),
        ..Default::default()
    };
    let planned = plan_audio(
        &AudioRequest {
            audio: by_id,
            engine: None,
            model: None,
            device: None,
            flash: false,
            speed: None,
            subtitles: None,
            provider: None,
            docs: None,
            output_dir: None,
            working_dir: None,
            continue_on_error: false,
            overwrite: false,
        },
        &data,
        Some(&store),
    )
    .unwrap();
    assert_eq!(planned.input.audio.as_deref(), Some(art_path.as_path()));
}

#[test]
fn audio_plan_has_job_shape() {
    let request = AudioRequest {
        audio: path_src("/tmp/audio.wav"),
        engine: None,
        model: None,
        device: None,
        flash: false,
        speed: None,
        subtitles: None,
        provider: None,
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
    assert_eq!(job.leaf_steps()[0].r#use, Capability::Preprocess);
}

#[test]
fn audio_plan_url_resolves_then_static_pipeline() {
    let dir = TempDir::new().unwrap();
    let request = AudioRequest {
        audio: InputSource {
            url: Some("https://example.com/x".into()),
            ..Default::default()
        },
        engine: None,
        model: None,
        device: None,
        flash: false,
        speed: None,
        subtitles: Some("prefer".into()),
        provider: Some("stub".into()),
        docs: None,
        output_dir: None,
        working_dir: None,
        continue_on_error: false,
        overwrite: true,
    };
    let job = plan_audio(&request, dir.path(), None).unwrap();
    assert!(job.input.audio.as_ref().unwrap().is_file());
    let leaves = job.leaf_steps();
    assert_eq!(leaves[0].r#use, Capability::Preprocess);
    assert!(!leaves.iter().any(|s| s.r#use == Capability::ImportUrl));
}

#[test]
fn meeting_plan_from_audio_convenience() {
    let request = MeetingPlanRequest {
        working_dir: Some(Path::new("/work").to_path_buf()),
        inputs: Vec::new(),
        meeting: Default::default(),
        output: Default::default(),
        audio: Some(path_src("/work/meeting.wav")),
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
            url: None,
            artifact: None,
            blob: None,
            participant: None,
            purposes: Vec::new(),
            subtitles: None,
            provider: None,
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

#[test]
fn meeting_plan_url_resolves_without_import_url_step() {
    let dir = TempDir::new().unwrap();
    let work = dir.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let request = MeetingPlanRequest {
        working_dir: Some(work.clone()),
        inputs: vec![MeetingInput {
            role: InputRole::Room,
            path: None,
            uri: None,
            url: Some("https://example.com/room".into()),
            artifact: None,
            blob: None,
            participant: None,
            purposes: Vec::new(),
            subtitles: None,
            provider: Some("stub".into()),
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
    let job = plan_meeting(&request, dir.path(), None).unwrap();
    assert!(!job
        .leaf_steps()
        .iter()
        .any(|s| s.r#use == Capability::ImportUrl));
    assert!(job.leaf_steps().iter().any(|s| s.r#use == Capability::Transcribe));
}
