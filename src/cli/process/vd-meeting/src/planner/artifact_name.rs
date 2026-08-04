//! Meeting artifact basename: `meeting_<YYYY-MM-DD>_<participants…>`.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::normalize::ResolvedMeeting;
use crate::model::InputRole;

/// Stem for `meeting_….json` / `meeting_….md` (no extension).
pub fn meeting_artifact_stem(resolved: &ResolvedMeeting) -> String {
    let date = meeting_date_label(resolved);
    let people = participant_labels(resolved);
    let mut parts = Vec::with_capacity(2 + people.len());
    parts.push("meeting".to_string());
    if let Some(d) = date {
        parts.push(d);
    }
    parts.extend(people);
    parts.join("_")
}

pub fn meeting_artifact_json_name(resolved: &ResolvedMeeting) -> String {
    format!("{}.json", meeting_artifact_stem(resolved))
}

fn meeting_date_label(resolved: &ResolvedMeeting) -> Option<String> {
    let mut earliest: Option<SystemTime> = None;
    for src in &resolved.inputs {
        if matches!(src.role, InputRole::Context) {
            continue;
        }
        if src.path.as_os_str().is_empty() {
            continue;
        }
        if let Some(t) = file_meeting_time(&src.path) {
            earliest = Some(match earliest {
                Some(prev) => prev.min(t),
                None => t,
            });
        }
    }
    let t = earliest?;
    format_ymd_utc(t)
}

fn file_meeting_time(path: &Path) -> Option<SystemTime> {
    let meta = fs::metadata(path).ok()?;
    meta.created().or_else(|_| meta.modified()).ok()
}

fn format_ymd_utc(t: SystemTime) -> Option<String> {
    let secs = t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// Howard Hinnant `civil_from_days` (UTC days since 1970-01-01 → Y-M-D).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn participant_labels(resolved: &ResolvedMeeting) -> Vec<String> {
    let mut labels = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for k in &resolved.meeting.participants.known {
        let raw = k.name.as_deref().or(k.id.as_deref()).unwrap_or("").trim();
        push_label(&mut labels, &mut seen, raw);
    }

    if !labels.is_empty() {
        return labels;
    }

    for src in &resolved.inputs {
        if src.role != InputRole::Participant {
            continue;
        }
        let raw = src
            .participant
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(src.branch_id.as_str());
        push_label(&mut labels, &mut seen, raw);
    }
    labels
}

fn push_label(labels: &mut Vec<String>, seen: &mut std::collections::BTreeSet<String>, raw: &str) {
    let label = sanitize_filename_part(raw);
    if label.is_empty() {
        return;
    }
    let key = label.to_lowercase();
    if seen.insert(key) {
        labels.push(label);
    }
}

fn sanitize_filename_part(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_sep = false;
    for ch in raw.chars() {
        let bad = matches!(
            ch,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t'
        ) || ch.is_control();
        if bad || ch == ' ' || ch == '_' {
            if !prev_sep && !out.is_empty() {
                out.push('-');
                prev_sep = true;
            }
            continue;
        }
        out.push(ch);
        prev_sep = false;
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.chars().count() > 48 {
        out = out.chars().take(48).collect();
        while out.ends_with('-') {
            out.pop();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{KnownParticipant, MeetingModel, MeetingOutput, Participants};
    use crate::planner::normalize::ResolvedInput;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn sanitize_keeps_cyrillic_and_strips_bad() {
        assert_eq!(sanitize_filename_part("Игорь"), "Игорь");
        assert_eq!(sanitize_filename_part("Владимир Языков"), "Владимир-Языков");
        assert_eq!(sanitize_filename_part("a/b:c"), "a-b-c");
    }

    #[test]
    fn civil_epoch_and_known_day() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 2026-07-31 00:00:00 UTC
        let days = 1_785_456_000i64 / 86_400;
        assert_eq!(civil_from_days(days), (2026, 7, 31));
    }

    #[test]
    fn stem_includes_date_and_names() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("igor.wav");
        let b = dir.path().join("vlad.wav");
        fs::write(&a, b"x").unwrap();
        fs::write(&b, b"x").unwrap();

        let resolved = ResolvedMeeting {
            working_dir: dir.path().to_path_buf(),
            inputs: vec![
                ResolvedInput {
                    role: InputRole::Participant,
                    path: a,
                    url: None,
                    subtitles: None,
                    participant: Some("igor".into()),
                    purposes: vec![],
                    branch_id: "igor".into(),
                    display_name: Some("Игорь".into()),
                },
                ResolvedInput {
                    role: InputRole::Participant,
                    path: b,
                    url: None,
                    subtitles: None,
                    participant: Some("vladimir".into()),
                    purposes: vec![],
                    branch_id: "vladimir".into(),
                    display_name: Some("Владимир".into()),
                },
            ],
            meeting: MeetingModel {
                participants: Participants {
                    known: vec![
                        KnownParticipant {
                            id: Some("igor".into()),
                            name: Some("Игорь".into()),
                            optional: false,
                            constraints: Default::default(),
                        },
                        KnownParticipant {
                            id: Some("vladimir".into()),
                            name: Some("Владимир".into()),
                            optional: false,
                            constraints: Default::default(),
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            },
            output: MeetingOutput::default(),
            has_room: false,
            has_context: false,
            text_sources: vec![],
            timeline_sources: vec![],
        };

        let stem = meeting_artifact_stem(&resolved);
        assert!(stem.starts_with("meeting_"), "stem={stem}");
        assert!(stem.contains("Игорь"), "stem={stem}");
        assert!(stem.contains("Владимир"), "stem={stem}");
        assert!(
            stem.split('_').any(|p| {
                p.len() == 10
                    && p.as_bytes().get(4) == Some(&b'-')
                    && p.as_bytes().get(7) == Some(&b'-')
                    && p.chars().filter(|c| c.is_ascii_digit()).count() == 8
            }),
            "expected date in stem={stem}"
        );
        assert_eq!(
            meeting_artifact_json_name(&resolved),
            format!("{stem}.json")
        );
    }
}
