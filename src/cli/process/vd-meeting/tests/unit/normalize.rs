//! Normalize + purposes + id generation.

use std::path::PathBuf;

use vd_meeting::{
    plan_job, BuildOptions, DiarizationEnabled, DiarizationPolicy, InputPurpose, InputRole,
    InputSource, MeetingModel, MeetingRequest, Participants,
};
use vd_pipeline::Capability;

fn src(role: InputRole, path: &str, participant: Option<&str>) -> InputSource {
    InputSource {
        role,
        path: PathBuf::from(path),
        url: None,
        participant: participant.map(str::to_string),
        purposes: Vec::new(),
        subtitles: None,
        provider: None,
    }
}

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
                src(InputRole::Participant, "alice.wav", Some("alice")),
                src(InputRole::Participant, "bob.wav", Some("bob")),
            ],
            DiarizationEnabled::Auto,
        ),
        &BuildOptions::default(),
    )
    .unwrap();

    let leaves = job.leaf_steps();
    assert!(!leaves.iter().any(|s| s.r#use == Capability::Diarize));
    assert!(leaves.iter().any(|s| s.r#use == Capability::MeetingMerge));
    let texts: Vec<_> = leaves
        .iter()
        .filter(|s| s.id.as_deref() == Some("alice.text") || s.id.as_deref() == Some("bob.text"))
        .collect();
    assert_eq!(texts.len(), 2);
}

#[test]
fn cyrillic_filename_keeps_cyrillic_branch_and_label() {
    let job = plan_job(
        &req(
            vec![
                src(InputRole::Participant, "Игорь.wav", None),
                src(InputRole::Participant, "Владимир.wav", None),
            ],
            DiarizationEnabled::False,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    let leaves = job.leaf_steps();
    assert!(
        leaves.iter().any(|s| s.id.as_deref() == Some("игорь.text")),
        "Cyrillic stem must become unicode branch id, not empty/track"
    );
    assert!(leaves
        .iter()
        .any(|s| s.id.as_deref() == Some("владимир.text")));
    let merge = leaves
        .iter()
        .find(|s| s.r#use == Capability::MeetingMerge)
        .expect("merge");
    let labels = merge
        .options
        .get("speaker_labels")
        .and_then(vd_pipeline::ArgValue::as_map)
        .expect("speaker_labels");
    assert_eq!(
        labels
            .get("игорь")
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref(),
        Some("Игорь")
    );
    assert_eq!(
        labels
            .get("владимир")
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref(),
        Some("Владимир")
    );
}

#[test]
fn latin_participant_id_still_uses_cyrillic_filename_display() {
    let job = plan_job(
        &req(
            vec![src(
                InputRole::Participant,
                "Игорь.wav",
                Some("igor"),
            )],
            DiarizationEnabled::False,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    let merge = job
        .leaf_steps()
        .into_iter()
        .find(|s| s.r#use == Capability::MeetingMerge)
        .expect("merge");
    let labels = merge
        .options
        .get("speaker_labels")
        .and_then(vd_pipeline::ArgValue::as_map)
        .expect("speaker_labels");
    // Branch id may stay Latin (explicit participant), display must stay Cyrillic.
    assert_eq!(
        labels
            .get("igor")
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref(),
        Some("Игорь")
    );
}

#[test]
fn room_alone_auto_transcript_and_diarize() {
    let job = plan_job(
        &req(
            vec![src(InputRole::Room, "meeting.wav", None)],
            DiarizationEnabled::Auto,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    assert!(job
        .leaf_steps()
        .iter()
        .any(|s| s.r#use == Capability::Diarize));
    assert!(job
        .leaf_steps()
        .iter()
        .any(|s| s.id.as_deref() == Some("room.text")));
}

#[test]
fn room_with_tracks_includes_room_transcript() {
    let job = plan_job(
        &req(
            vec![
                src(InputRole::Room, "meeting.wav", None),
                src(InputRole::Participant, "alice.wav", Some("alice")),
                src(InputRole::Participant, "bob.wav", Some("bob")),
            ],
            DiarizationEnabled::Auto,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    let leaves = job.leaf_steps();
    assert!(leaves.iter().any(|s| s.r#use == Capability::Diarize));
    assert!(
        leaves.iter().any(|s| s.id.as_deref() == Some("room.text")),
        "ADR 0016: room+tracks ASR the mix"
    );
    assert!(leaves.iter().any(|s| s.id.as_deref() == Some("alice.text")));
    assert!(leaves.iter().any(|s| s.id.as_deref() == Some("bob.text")));
}

#[test]
fn room_explicit_transcript_with_tracks() {
    let mut room = src(InputRole::Room, "meeting.wav", None);
    room.purposes = vec![InputPurpose::Transcript, InputPurpose::Timeline];
    let job = plan_job(
        &req(
            vec![
                room,
                src(InputRole::Participant, "alice.wav", Some("alice")),
            ],
            DiarizationEnabled::Auto,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    assert!(job
        .leaf_steps()
        .iter()
        .any(|s| s.id.as_deref() == Some("room.text")));
}

#[test]
fn diarization_false_skips() {
    let job = plan_job(
        &req(
            vec![src(InputRole::Room, "meeting.wav", None)],
            DiarizationEnabled::False,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    assert!(!job
        .leaf_steps()
        .iter()
        .any(|s| s.r#use == Capability::Diarize));
}

#[test]
fn context_adds_prepare() {
    let job = plan_job(
        &req(
            vec![
                src(InputRole::Room, "meeting.wav", None),
                src(InputRole::Context, "docs", None),
            ],
            DiarizationEnabled::False,
        ),
        &BuildOptions::default(),
    )
    .unwrap();
    assert_eq!(job.leaf_steps()[0].r#use, Capability::PrepareContext);
    assert!(job.context.docs.is_some());
}
