//! Normalize inputs + Meeting Model → ResolvedMeeting.
//!
//! Flow: resolve purposes → collect required artifacts → branch ids.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::PlanError;
use crate::model::{
    DiarizationEnabled, InputPurpose, InputRole, InputSource, MeetingModel, MeetingOutput,
    MeetingRequest, Participants,
};

#[derive(Debug, Clone)]
pub struct ResolvedMeeting {
    pub working_dir: PathBuf,
    pub inputs: Vec<ResolvedInput>,
    pub meeting: MeetingModel,
    pub output: MeetingOutput,
    pub has_room: bool,
    pub has_context: bool,
    /// Indices into `inputs` that need a transcript branch.
    pub text_sources: Vec<usize>,
    /// Indices into `inputs` that may feed diarization / timeline.
    pub timeline_sources: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct ResolvedInput {
    pub role: InputRole,
    pub path: PathBuf,
    pub url: Option<String>,
    pub subtitles: Option<String>,
    pub participant: Option<String>,
    pub purposes: Vec<InputPurpose>,
    /// Stable branch id (alice, bob, room, track-0, …).
    pub branch_id: String,
    /// Human label for transcripts (original script/casing from file or `participant`).
    /// Prefer this over raw `branch_id` when `participants.known` has no `name`.
    pub display_name: Option<String>,
}

pub fn normalize(request: &MeetingRequest) -> Result<ResolvedMeeting, PlanError> {
    if request.inputs.is_empty() {
        return Err(PlanError::Usage("meeting has no inputs".into()));
    }

    let working_dir = request
        .working_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    let mut meeting = request.meeting.clone();
    normalize_participants(&mut meeting.participants)?;

    let has_participant = request
        .inputs
        .iter()
        .any(|s| s.role == InputRole::Participant);
    let diarization = meeting.diarization.enabled;

    let mut used_ids: HashSet<String> = HashSet::new();
    let mut inputs = Vec::with_capacity(request.inputs.len());
    let mut track_idx = 0u32;
    let mut has_room = false;
    let mut has_context = false;
    let mut text_sources = Vec::new();
    let mut timeline_sources = Vec::new();

    for (i, src) in request.inputs.iter().enumerate() {
        validate_source(src)?;
        let url = src
            .url
            .as_deref()
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .map(str::to_string);
        let path = if url.is_some() {
            PathBuf::new()
        } else {
            resolve_path(&working_dir, &src.path)
        };
        let purposes = resolve_purposes(src, has_participant, diarization)?;

        let branch_id = match src.role {
            InputRole::Room => {
                has_room = true;
                unique_id("room", &mut used_ids)
            }
            InputRole::Participant => {
                let base = src
                    .participant
                    .clone()
                    .or_else(|| {
                        if let Some(u) = &url {
                            Some(slugify(u))
                        } else {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .map(str::to_string)
                        }
                    })
                    .unwrap_or_else(|| {
                        let id = format!("track-{track_idx}");
                        track_idx += 1;
                        id
                    });
                sanitize_id(&base, &mut used_ids)
            }
            InputRole::Context => {
                has_context = true;
                unique_id("context", &mut used_ids)
            }
        };

        let display_name = match src.role {
            InputRole::Participant => {
                let from_participant = src
                    .participant
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                let from_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                // Prefer the label that keeps the original script (Игорь.wav +
                // participant=igor → display Игорь). Latin ids stay fine for wiring.
                prefer_original_script(from_participant, from_stem)
            }
            _ => None,
        };

        if purposes.contains(&InputPurpose::Transcript) {
            text_sources.push(i);
        }
        if purposes.contains(&InputPurpose::Timeline) {
            timeline_sources.push(i);
        }

        inputs.push(ResolvedInput {
            role: src.role,
            path,
            url,
            subtitles: src.subtitles.clone(),
            participant: src.participant.clone(),
            purposes,
            branch_id,
            display_name,
        });
    }

    if text_sources.is_empty() {
        return Err(PlanError::Usage(
            "need at least one audio input with purpose transcript \
             (participant track, or room with purposes including transcript)"
                .into(),
        ));
    }

    Ok(ResolvedMeeting {
        working_dir,
        inputs,
        meeting,
        output: request.output.clone(),
        has_room,
        has_context,
        text_sources,
        timeline_sources,
    })
}

/// Default purposes when the document omits them.
///
/// | Role | Context | Default |
/// |------|---------|---------|
/// | participant | any | `[transcript]` |
/// | room | with participant tracks | `[transcript, timeline]` (ADR 0016: mix ASR + diarize) |
/// | room | alone, diarization on/auto | `[transcript, timeline]` |
/// | room | alone, diarization off | `[transcript]` |
/// | context | any | `[]` |
fn resolve_purposes(
    src: &InputSource,
    has_participant: bool,
    diarization: DiarizationEnabled,
) -> Result<Vec<InputPurpose>, PlanError> {
    if !src.purposes.is_empty() {
        if src.role == InputRole::Context {
            return Err(PlanError::Usage(
                "context inputs cannot declare audio purposes".into(),
            ));
        }
        // Dedup while preserving order.
        let mut out = Vec::new();
        for p in &src.purposes {
            if !out.contains(p) {
                out.push(*p);
            }
        }
        return Ok(out);
    }

    Ok(match src.role {
        InputRole::Participant => vec![InputPurpose::Transcript],
        InputRole::Context => Vec::new(),
        InputRole::Room => {
            if has_participant {
                // ADR 0016: room mix ASR (for subtract) + timeline (diarize).
                vec![InputPurpose::Transcript, InputPurpose::Timeline]
            } else if matches!(
                diarization,
                DiarizationEnabled::True | DiarizationEnabled::Auto
            ) {
                vec![InputPurpose::Transcript, InputPurpose::Timeline]
            } else {
                vec![InputPurpose::Transcript]
            }
        }
    })
}

/// Fail if audio / context paths are missing (CLI run / e2e).
pub fn require_paths(resolved: &ResolvedMeeting) -> Result<(), PlanError> {
    for src in &resolved.inputs {
        if src.url.is_some() {
            continue;
        }
        if !src.path.exists() {
            return Err(PlanError::NotFound(format!(
                "input missing: {}",
                src.path.display()
            )));
        }
    }
    Ok(())
}

fn validate_source(src: &InputSource) -> Result<(), PlanError> {
    let has_url = src
        .url
        .as_deref()
        .map(str::trim)
        .is_some_and(|u| !u.is_empty());
    let has_path = !src.path.as_os_str().is_empty();
    match (has_path, has_url) {
        (true, false) | (false, true) => {}
        (false, false) => {
            return Err(PlanError::Usage("input needs path=… or url=…".into()));
        }
        (true, true) => {
            return Err(PlanError::Usage(
                "input must not set both path and url".into(),
            ));
        }
    }
    if has_url && src.role == InputRole::Context {
        return Err(PlanError::Usage("context inputs cannot use url".into()));
    }
    if src.role == InputRole::Context && !src.purposes.is_empty() {
        return Err(PlanError::Usage(
            "context inputs cannot declare audio purposes".into(),
        ));
    }
    Ok(())
}

fn normalize_participants(p: &mut Participants) -> Result<(), PlanError> {
    let mut used = HashSet::new();
    for (i, k) in p.known.iter_mut().enumerate() {
        if k.name.is_none() && k.id.is_none() {
            return Err(PlanError::Usage(
                "known participant needs id or name".into(),
            ));
        }
        if k.id.is_none() {
            let base = k
                .name
                .as_deref()
                .map(slugify)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("p{i}"));
            k.id = Some(unique_id(&base, &mut used));
        } else if let Some(id) = &k.id {
            if !used.insert(id.clone()) {
                return Err(PlanError::Usage(format!(
                    "duplicate known participant id: {id}"
                )));
            }
        }
    }
    if let Some(c) = &p.constraints {
        validate_count_pair(c.min, c.max, "participants.constraints")?;
        for (g, b) in &c.genders {
            validate_count_pair(b.min, b.max, &format!("genders.{}", g.as_str()))?;
        }
    }
    if let Some(e) = &p.expected {
        validate_count_pair(e.min, e.max, "participants.expected")?;
    }
    Ok(())
}

fn validate_count_pair(min: Option<u32>, max: Option<u32>, ctx: &str) -> Result<(), PlanError> {
    if let (Some(a), Some(b)) = (min, max) {
        if a > b {
            return Err(PlanError::Usage(format!("{ctx}: min > max")));
        }
    }
    Ok(())
}

fn resolve_path(base: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

fn has_non_ascii_letter(s: &str) -> bool {
    s.chars().any(|c| c.is_alphabetic() && !c.is_ascii())
}

/// Prefer a non-ASCII (e.g. Cyrillic) human label over an ASCII slug when both exist.
fn prefer_original_script(primary: Option<&str>, fallback: Option<&str>) -> Option<String> {
    match (primary, fallback) {
        (Some(a), Some(b)) => {
            let a_native = has_non_ascii_letter(a);
            let b_native = has_non_ascii_letter(b);
            if !a_native && b_native {
                Some(b.to_string())
            } else {
                Some(a.to_string())
            }
        }
        (Some(a), None) => Some(a.to_string()),
        (None, Some(b)) => Some(b.to_string()),
        (None, None) => None,
    }
}

fn slugify(s: &str) -> String {
    // Keep letters/digits from any script (Игорь → игорь). ASCII-only was wrong:
    // Cyrillic names collapsed to empty → "track" / forced Latin branch ids in transcripts.
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn sanitize_id(base: &str, used: &mut HashSet<String>) -> String {
    let slug = slugify(base);
    let slug = if slug.is_empty() {
        "track".into()
    } else {
        slug
    };
    unique_id(&slug, used)
}

fn unique_id(base: &str, used: &mut HashSet<String>) -> String {
    if !used
        .iter()
        .any(|u| u.to_lowercase() == base.to_lowercase())
    {
        used.insert(base.to_string());
        return base.to_string();
    }
    for n in 2..10_000 {
        let cand = format!("{base}-{n}");
        if !used
            .iter()
            .any(|u| u.to_lowercase() == cand.to_lowercase())
        {
            used.insert(cand.clone());
            return cand;
        }
    }
    let fallback = format!("{base}-x");
    used.insert(fallback.clone());
    fallback
}
