//! plan_job → Job validates via vd-pipeline resolve.

use std::path::PathBuf;

use vd_meeting::{
    plan_job, BuildOptions, DiarizationEnabled, DiarizationPolicy, InputRole, InputSource,
    KnownParticipant, MeetingModel, MeetingRequest, ParticipantConstraints, Participants, Gender,
};
use vd_pipeline::{resolve_job, Capability};

#[test]
fn merged_plus_tracks_resolves() {
    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![
            InputSource {
                role: InputRole::Merged,
                path: PathBuf::from("meeting.wav"),
                participant: None,
            },
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
    assert!(job.steps.iter().any(|s| s.r#use == Capability::Diarize));
    let merge = job
        .steps
        .iter()
        .find(|s| s.r#use == Capability::MeetingMerge)
        .unwrap();
    assert!(merge.inputs.iter().any(|i| i == "timeline"));
    assert!(merge.inputs.iter().any(|i| i == "alice.text"));
    assert!(merge.inputs.iter().any(|i| i == "bob.text"));

    resolve_job(job).expect("planned Job must resolve");
}
