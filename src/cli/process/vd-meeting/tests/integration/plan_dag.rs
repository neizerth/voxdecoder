//! plan_job → Job validates via vd-pipeline resolve.

use std::path::PathBuf;

use vd_meeting::{
    plan_job, BuildOptions, DiarizationEnabled, DiarizationPolicy, Gender, InputRole, InputSource,
    KnownParticipant, MeetingModel, MeetingRequest, ParticipantConstraints, Participants,
};
use vd_pipeline::{resolve_job, Capability};

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

#[test]
fn room_plus_tracks_resolves_without_room_transcript() {
    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![
            src(InputRole::Room, "meeting.wav", None),
            src(InputRole::Participant, "alice.wav", Some("alice")),
            src(InputRole::Participant, "bob.wav", Some("bob")),
        ],
        meeting: MeetingModel {
            participants: Participants {
                known: vec![
                    KnownParticipant {
                        id: Some("alice".into()),
                        name: Some("Alice".into()),
                        optional: false,
                        constraints: ParticipantConstraints {
                            gender: Some(Gender::Female),
                            ..Default::default()
                        },
                    },
                    KnownParticipant {
                        id: Some("bob".into()),
                        name: Some("Bob".into()),
                        optional: false,
                        constraints: ParticipantConstraints {
                            gender: Some(Gender::Male),
                            ..Default::default()
                        },
                    },
                ],
                expected: None,
                constraints: None,
            },
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::Auto,
            },
            alignment: Default::default(),
        },
        output: Default::default(),
    };

    let job = plan_job(&req, &BuildOptions::default()).unwrap();
    let leaves = job.leaf_steps();
    assert!(leaves.iter().any(|s| s.r#use == Capability::Diarize));
    assert!(!leaves.iter().any(|s| s.id.as_deref() == Some("room.text")));
    let merge = leaves
        .iter()
        .find(|s| s.r#use == Capability::MeetingMerge)
        .unwrap();
    assert!(merge.inputs.iter().any(|i| i == "timeline"));
    assert!(merge.inputs.iter().any(|i| i == "alice.text"));
    assert!(merge.inputs.iter().any(|i| i == "bob.text"));
    assert!(!merge.inputs.iter().any(|i| i == "room.text"));
    // Diarize mode: mix is not attached separately (timeline covers room).
    assert!(merge.options.get("mix").is_none());

    resolve_job(job).expect("planned Job must resolve");
}

#[test]
fn diarize_false_with_tracks_attaches_mix_not_timeline() {
    use vd_meeting::{AlignmentOptions, AlignmentReference};

    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![
            src(InputRole::Room, "meeting.wav", None),
            src(InputRole::Participant, "alice.wav", Some("alice")),
            src(InputRole::Participant, "bob.wav", Some("bob")),
        ],
        meeting: MeetingModel {
            participants: Default::default(),
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::False,
            },
            alignment: AlignmentOptions {
                reference: AlignmentReference::Auto,
                ..Default::default()
            },
        },
        output: Default::default(),
    };

    let job = plan_job(&req, &BuildOptions::default()).unwrap();
    let leaves = job.leaf_steps();
    assert!(!leaves.iter().any(|s| s.r#use == Capability::Diarize));
    let merge = leaves
        .iter()
        .find(|s| s.r#use == Capability::MeetingMerge)
        .expect("meeting-merge");
    assert!(merge.inputs.iter().any(|i| i == "alice.text"));
    assert!(merge.inputs.iter().any(|i| i == "bob.text"));
    assert!(!merge.inputs.iter().any(|i| i == "timeline"));
    let mix = merge
        .options
        .get("mix")
        .and_then(vd_pipeline::ArgValue::as_string)
        .expect("mix reference");
    assert!(
        mix.contains("meeting.wav") || mix == "room.prepared" || mix.ends_with("meeting.wav"),
        "unexpected mix ref: {mix}"
    );
    assert!(merge.inputs.iter().any(|i| i == &mix));
    let reference = merge
        .options
        .get("alignment")
        .and_then(vd_pipeline::ArgValue::as_map)
        .and_then(|m| m.get("reference"))
        .and_then(vd_pipeline::ArgValue::as_string);
    assert_eq!(reference.as_deref(), Some("auto"));

    resolve_job(job).expect("planned Job must resolve");
}

#[test]
fn alignment_reference_none_ignores_mix() {
    use vd_meeting::{AlignmentOptions, AlignmentReference};

    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![
            src(InputRole::Room, "meeting.wav", None),
            src(InputRole::Participant, "alice.wav", Some("alice")),
        ],
        meeting: MeetingModel {
            participants: Default::default(),
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::False,
            },
            alignment: AlignmentOptions {
                reference: AlignmentReference::None,
                ..Default::default()
            },
        },
        output: Default::default(),
    };

    let job = plan_job(&req, &BuildOptions::default()).unwrap();
    let merge = job
        .leaf_steps()
        .into_iter()
        .find(|s| s.r#use == Capability::MeetingMerge)
        .unwrap();
    assert!(merge.options.get("mix").is_none());
    assert!(!merge.inputs.iter().any(|i| i.contains("meeting.wav")));
    resolve_job(job).expect("planned Job must resolve");
}

#[test]
fn video_room_inserts_preprocess_before_diarize() {
    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![
            src(InputRole::Room, "meeting.mp4", None),
            src(InputRole::Participant, "alice.wav", Some("alice")),
        ],
        meeting: MeetingModel {
            participants: Participants {
                known: vec![KnownParticipant {
                    id: Some("alice".into()),
                    name: Some("Alice".into()),
                    optional: false,
                    constraints: Default::default(),
                }],
                expected: None,
                constraints: None,
            },
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::Auto,
            },
            alignment: Default::default(),
        },
        output: Default::default(),
    };

    let job = plan_job(&req, &BuildOptions::default()).unwrap();
    let leaves = job.leaf_steps();
    let prep = leaves
        .iter()
        .find(|s| s.r#use == Capability::Preprocess)
        .expect("video room should preprocess");
    assert_eq!(prep.id.as_deref(), Some("room.prepared"));
    let first = prep
        .options
        .get("filters")
        .and_then(vd_pipeline::ArgValue::as_list)
        .and_then(|f| f.first())
        .and_then(|f| f.as_map())
        .and_then(|m| m.get("type"))
        .and_then(vd_pipeline::ArgValue::as_string);
    assert_eq!(first.as_deref(), Some("extract-audio"));

    let diarize = leaves
        .iter()
        .find(|s| s.r#use == Capability::Diarize)
        .expect("diarize");
    assert_eq!(diarize.input.as_deref(), Some("room.prepared"));

    resolve_job(job).expect("planned Job must resolve");
}
