//! Default Job from CLI flags.

use std::path::PathBuf;

use super::fixture;
use vd_pipeline::{default_job, load_job_file, Capability, DefaultJobArgs, TranscribeEngine};

fn base_args() -> DefaultJobArgs {
    DefaultJobArgs {
        audio: PathBuf::from("meeting.ogg"),
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
    }
}

#[test]
fn default_matches_fixture() {
    let mut expected = load_job_file(&fixture("jobs/default.yaml")).unwrap();
    #[cfg(target_os = "macos")]
    {
        // macOS defaults transcribe device to metal when unset.
        for node in &mut expected.steps {
            if let vd_pipeline::WorkflowNode::Step(step) = node {
                if step.r#use == Capability::Transcribe {
                    step.options.insert(
                        "device".into(),
                        vd_pipeline::ArgValue::String("metal".into()),
                    );
                }
            }
        }
    }
    let got = default_job(&base_args());
    assert_eq!(got, expected);
}

#[test]
fn default_always_includes_prepare_context() {
    let job = default_job(&base_args());
    assert!(job
        .leaf_steps()
        .iter()
        .any(|s| s.r#use == Capability::PrepareContext));
    assert_eq!(job.context.docs, Some(PathBuf::from(".")));
}

#[test]
fn docs_override_sets_context() {
    let mut args = base_args();
    args.docs = Some(PathBuf::from("./docs"));
    let job = default_job(&args);
    assert!(job
        .leaf_steps()
        .iter()
        .any(|s| s.r#use == Capability::PrepareContext));
    assert_eq!(job.context.docs, Some(PathBuf::from("./docs")));
}

#[test]
fn speed_inserts_preprocess_filter() {
    let mut args = base_args();
    args.speed = Some(2.0);
    let job = default_job(&args);
    let prep = job
        .leaf_steps()
        .into_iter()
        .find(|s| s.r#use == Capability::Preprocess)
        .expect("preprocess");
    let filters = prep
        .options
        .get("filters")
        .and_then(vd_pipeline::ArgValue::as_list)
        .expect("filters list");
    let has_speed = filters.iter().any(|f| {
        f.as_map()
            .and_then(|m| m.get("type"))
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref()
            == Some("speed")
    });
    assert!(has_speed, "expected speed filter in preprocess chain");
}

#[test]
fn video_input_prepends_extract_audio_with_ffmpeg() {
    let mut args = base_args();
    args.audio = PathBuf::from("meeting.mp4");
    let job = default_job(&args);
    let prep = job
        .leaf_steps()
        .into_iter()
        .find(|s| s.r#use == Capability::Preprocess)
        .expect("preprocess");
    assert_eq!(
        prep.options
            .get("provider")
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref(),
        Some("ffmpeg")
    );
    let filters = prep
        .options
        .get("filters")
        .and_then(vd_pipeline::ArgValue::as_list)
        .expect("filters list");
    let first = filters[0]
        .as_map()
        .and_then(|m| m.get("type"))
        .and_then(vd_pipeline::ArgValue::as_string);
    assert_eq!(first.as_deref(), Some("extract-audio"));
}

#[test]
fn is_video_path_detects_containers() {
    assert!(vd_pipeline::is_video_path(std::path::Path::new("a.MP4")));
    assert!(vd_pipeline::is_video_path(std::path::Path::new("a.mkv")));
    assert!(!vd_pipeline::is_video_path(std::path::Path::new("a.wav")));
    assert!(!vd_pipeline::is_video_path(std::path::Path::new("a.m4a")));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_defaults_device_metal() {
    let job = default_job(&base_args());
    let tx = job
        .leaf_steps()
        .into_iter()
        .find(|s| s.r#use == Capability::Transcribe)
        .expect("transcribe");
    assert_eq!(
        tx.options
            .get("device")
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref(),
        Some("metal")
    );
}
