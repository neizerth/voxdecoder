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
    pub participant: Option<String>,
    pub purposes: Vec<InputPurpose>,
    /// Stable branch id (alice, bob, room, track-0, …).
    pub branch_id: String,
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
        let path = resolve_path(&working_dir, &src.path);
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
                        path.file_stem()
                            .and_then(|s| s.to_str())
                            .map(str::to_string)
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

        if purposes.contains(&InputPurpose::Transcript) {
            text_sources.push(i);
        }
        if purposes.contains(&InputPurpose::Timeline) {
            timeline_sources.push(i);
        }

        inputs.push(ResolvedInput {
            role: src.role,
            path,
            participant: src.participant.clone(),
            purposes,
            branch_id,
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
/// | room | with participant tracks | `[timeline]` (mix for diarize only) |
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
                vec![InputPurpose::Timeline]
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
    if src.path.as_os_str().is_empty() {
        return Err(PlanError::Usage("input path is empty".into()));
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

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
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
    if used.insert(base.to_string()) {
        return base.to_string();
    }
    for n in 2..10_000 {
        let cand = format!("{base}-{n}");
        if used.insert(cand.clone()) {
            return cand;
        }
    }
    format!("{base}-x")
}
