//! Normalize + id generation.

use std::path::PathBuf;

use vd_meeting::{
    plan_job, BuildOptions, DiarizationEnabled, DiarizationPolicy, InputRole, InputSource,
    MeetingModel, MeetingRequest, Participants,
};
use vd_pipeline::Capability;

fn req(inputs: Vec<InputSource>, diarization: DiarizationEnabled) -> MeetingRequest {
    MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs,
        meeting: MeetingModel {
            participants: Participants::default(),
            diarization: DiarizationPolicy {
                enabled: diarization,
            },
            alignment: Default::default(),
        },
        output: Default::default(),
    }
}

#[test]
fn participant_tracks_no_diarize() {
    let job = plan_job(
        &req(
            vec![
                InputSource {
                    role: InputRole::Participant,
                    path: PathBuf::from("alice.wav"),
                    participant: Some("alice".into()),
                },
                InputSource {
                    role: InputRole::Participant,
                    path: PathBuf::from("bob.wav"),
                    participant: Some("bob".into()),
                },
            ],
            DiarizationEnabled::Auto,
        ),
        &BuildOptions::default(),
    )
    .unwrap();

    assert!(!job
        .steps
        .iter()
        .any(|s| s.r#use == Capability::Diarize));
    assert!(job
        .steps
        .iter()
        .any(|s| s.r#use == Capability::MeetingMerge));
    let texts: Vec<_> = job
        .steps
        .iter()
        .filter(|s| s.id.as_deref() == Some("alice.text") || s.id.as_deref() == Some("bob.text"))
        .collect();
    assert_eq!(texts.len(), 2);
}

#[test]
fn merged_auto_adds_diarize() {
    let job = plan_job(
        &req(
            vec![InputSource {
                role: InputRole::Merged,
                path: PathBuf::from("meeting.wav"),
                participant: None,
            }],
            DiarizationEnabled::Auto,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    assert!(job.steps.iter().any(|s| s.r#use == Capability::Diarize));
    assert!(job
        .steps
        .iter()
        .any(|s| s.id.as_deref() == Some("merged.text")));
}

#[test]
fn diarization_false_skips() {
    let job = plan_job(
        &req(
            vec![InputSource {
                role: InputRole::Merged,
                path: PathBuf::from("meeting.wav"),
                participant: None,
            }],
            DiarizationEnabled::False,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    assert!(!job.steps.iter().any(|s| s.r#use == Capability::Diarize));
}

#[test]
fn context_adds_prepare() {
    let job = plan_job(
        &req(
            vec![
                InputSource {
                    role: InputRole::Merged,
                    path: PathBuf::from("meeting.wav"),
                    participant: None,
                },
                InputSource {
                    role: InputRole::Context,
                    path: PathBuf::from("docs"),
                    participant: None,
                },
            ],
            DiarizationEnabled::False,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    assert_eq!(job.steps[0].r#use, Capability::PrepareContext);
    assert!(job.context.docs.is_some());
}
