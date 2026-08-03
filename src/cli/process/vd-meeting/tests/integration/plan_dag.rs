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
fn diarized_meeting_appends_fix_overlap_after_merge() {
    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![
            src(InputRole::Room, "meeting.wav", None),
            src(InputRole::Participant, "alice.wav", Some("alice")),
            src(InputRole::Participant, "bob.wav", Some("bob")),
        ],
        meeting: MeetingModel {
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::Auto,
            },
            ..Default::default()
        },
        output: Default::default(),
    };

    let job = plan_job(&req, &BuildOptions::default()).unwrap();
    let leaves = job.leaf_steps();
    let overlap = leaves
        .iter()
        .find(|s| s.r#use == Capability::FixOverlap)
        .expect("diarized meeting should get a fix-overlap step");
    assert!(overlap.inputs.iter().any(|i| i == "meeting"));

    let merge = leaves
        .iter()
        .find(|s| s.r#use == Capability::MeetingMerge)
        .unwrap();
    assert_eq!(
        overlap.output, merge.output,
        "fix-overlap must rewrite the same well-known meeting artifact path, not a new file"
    );

    resolve_job(job).expect("planned Job must resolve");
}

#[test]
fn single_speaker_meeting_has_no_fix_overlap_step() {
    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![src(InputRole::Room, "meeting.wav", None)],
        meeting: MeetingModel {
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::False,
            },
            ..Default::default()
        },
        output: Default::default(),
    };

    let job = plan_job(&req, &BuildOptions::default()).unwrap();
    let leaves = job.leaf_steps();
    assert!(
        !leaves.iter().any(|s| s.r#use == Capability::FixOverlap),
        "single-speaker meetings have nothing to dedup"
    );

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

#[test]
fn speed_inserts_preprocess_on_audio_room() {
    let req = MeetingRequest {
        working_dir: Some(PathBuf::from("/work")),
        inputs: vec![src(InputRole::Room, "meeting.wav", None)],
        meeting: MeetingModel {
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::False,
            },
            ..Default::default()
        },
        output: Default::default(),
    };
    let mut options = BuildOptions::default();
    options.transcribe.speed = Some(2.0);
    let job = plan_job(&req, &options).unwrap();
    let leaves = job.leaf_steps();
    let prep = leaves
        .iter()
        .find(|s| s.r#use == Capability::Preprocess)
        .expect("speed should force preprocess on audio");
    let filters = prep
        .options
        .get("filters")
        .and_then(vd_pipeline::ArgValue::as_list)
        .expect("filters");
    let has_speed = filters.iter().any(|f| {
        f.as_map()
            .and_then(|m| m.get("type").or_else(|| m.get("operation")))
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref()
            == Some("speed")
    });
    assert!(has_speed, "expected speed filter");
    resolve_job(job).expect("planned Job must resolve");
}

/// Minimal mono PCM16 WAV so ffprobe can read duration.
fn write_wav(path: &std::path::Path, duration_sec: f64, sample_rate: u32) {
    let n = (duration_sec * f64::from(sample_rate)).round() as u32;
    let data_bytes = n * 2;
    let mut buf = Vec::with_capacity(44 + data_bytes as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36u32 + data_bytes).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes.to_le_bytes());
    buf.resize(buf.len() + data_bytes as usize, 0);
    std::fs::write(path, buf).unwrap();
}

#[test]
fn longest_alignment_pads_shorter_track() {
    if std::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("skip longest_alignment_pads_shorter_track: ffprobe missing");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let long_p = dir.path().join("long.wav");
    let short_p = dir.path().join("short.wav");
    write_wav(&long_p, 4.0, 16_000);
    write_wav(&short_p, 1.0, 16_000);

    let req = MeetingRequest {
        working_dir: Some(dir.path().to_path_buf()),
        inputs: vec![
            src(
                InputRole::Participant,
                long_p.to_str().unwrap(),
                Some("long"),
            ),
            src(
                InputRole::Participant,
                short_p.to_str().unwrap(),
                Some("short"),
            ),
        ],
        meeting: MeetingModel {
            diarization: DiarizationPolicy {
                enabled: DiarizationEnabled::False,
            },
            alignment: Default::default(), // mode: longest
            ..Default::default()
        },
        output: Default::default(),
    };

    let job = plan_job(&req, &BuildOptions::default()).unwrap();
    let leaves = job.leaf_steps();

    let short_prep = leaves
        .iter()
        .find(|s| s.id.as_deref() == Some("short.prepared"))
        .expect("short track should preprocess with pad-start");
    assert_eq!(
        short_prep
            .options
            .get("provider")
            .and_then(vd_pipeline::ArgValue::as_string)
            .as_deref(),
        Some("ffmpeg")
    );
    let filters = short_prep
        .options
        .get("filters")
        .and_then(vd_pipeline::ArgValue::as_list)
        .expect("filters");
    let pad = filters.iter().find_map(|f| {
        let m = f.as_map()?;
        let t = m
            .get("type")
            .or_else(|| m.get("operation"))
            .and_then(vd_pipeline::ArgValue::as_string)?;
        if t != "pad-start" {
            return None;
        }
        match m.get("duration_sec")? {
            vd_pipeline::ArgValue::Number(n) => Some(*n),
            other => other.as_string()?.parse().ok(),
        }
    });
    let pad = pad.expect("pad-start filter");
    assert!((pad - 3.0).abs() < 0.15, "expected ~3s pad, got {pad}");

    // Longest track needs no pad (no preprocess unless other reasons).
    assert!(
        !leaves
            .iter()
            .any(|s| s.id.as_deref() == Some("long.prepared")),
        "longest track should not get a pad preprocess"
    );

    // pad-start must sit after trim-silence in the chain.
    let types: Vec<_> = filters
        .iter()
        .filter_map(|f| {
            f.as_map()
                .and_then(|m| m.get("type").or_else(|| m.get("operation")))
                .and_then(vd_pipeline::ArgValue::as_string)
        })
        .collect();
    let trim_i = types.iter().position(|t| *t == "trim-silence");
    let pad_i = types.iter().position(|t| *t == "pad-start");
    assert!(
        matches!((trim_i, pad_i), (Some(t), Some(p)) if p > t),
        "pad-start should follow trim-silence: {types:?}"
    );

    resolve_job(job).expect("planned Job must resolve");
}
