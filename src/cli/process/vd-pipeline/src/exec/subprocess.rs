//! Spawn child CLIs for capabilities.

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::job::{is_video_path, ArgValue, Capability};

use super::{Binder, ExecError, InvokeRequest, InvokeResult};

#[derive(Debug, Default)]
pub struct SubprocessBinder;

impl Binder for SubprocessBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        match req.capability {
            Capability::Transcribe => run_transcribe(req),
            Capability::PrepareContext => run_prepare_context(req),
            Capability::FixCasing => run_fix(req, "vd-fix-casing"),
            Capability::FixAsr => run_fix(req, "vd-fix-asr"),
            Capability::FixDisfluency => run_fix(req, "vd-fix-disfluency"),
            Capability::FixTerms => run_fix(req, "vd-fix-terms"),
            Capability::FixLayout => run_fix(req, "vd-fix-layout"),
            Capability::FixOverlap => run_fix_overlap(req),
            Capability::Diarize => run_diarize(req),
            Capability::MeetingMerge => run_meeting_merge(req),
            Capability::Preprocess => run_preprocess(req),
            Capability::Postprocess => run_postprocess(req),
            Capability::ImportUrl => run_import_url(req),
        }
    }
}

fn run_import_url(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-url")?;
    let url = req
        .options
        .get("url")
        .and_then(ArgValue::as_string)
        .or_else(|| {
            let s = req.input.to_string_lossy();
            if s.starts_with("http://") || s.starts_with("https://") {
                Some(s.into_owned())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            ExecError::Step("import-url requires options.url or http(s) input".into())
        })?;

    let out_dir = req
        .output_dir
        .clone()
        .or_else(|| {
            req.output
                .as_ref()
                .and_then(|p| p.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| req.working_dir.join("import"));

    let mut args = vec![
        "run".into(),
        "-q".into(),
        "-i".into(),
        url,
        "--output-dir".into(),
        out_dir.display().to_string(),
        "--overwrite".into(),
    ];

    if let Some(s) = req.options.get("subtitles").and_then(ArgValue::as_string) {
        args.push("--subtitles".into());
        args.push(s);
    }
    if let Some(p) = req.options.get("provider").and_then(ArgValue::as_string) {
        args.push("--provider".into());
        args.push(p);
    }
    if req.options.get("metadata_only").and_then(ArgValue::as_bool) == Some(true)
        || req.options.get("metadata-only").and_then(ArgValue::as_bool) == Some(true)
    {
        args.push("--metadata-only".into());
    }

    run_cmd(&bin, &args, req, None)?;

    let audio = [
        "audio.m4a",
        "audio.wav",
        "audio.mp3",
        "audio.ogg",
        "audio.opus",
    ]
    .iter()
    .map(|n| out_dir.join(n))
    .find(|p| p.is_file());
    let metadata = out_dir.join("metadata.yaml");
    let subtitle = out_dir.join("subtitles.vtt");

    let primary = audio
        .clone()
        .or_else(|| metadata.is_file().then_some(metadata.clone()))
        .ok_or_else(|| ExecError::Step("import-url produced no artifacts".into()))?;

    let mut outputs = BTreeMap::new();
    if metadata.is_file() {
        outputs.insert("metadata".into(), metadata);
    }
    if subtitle.is_file() {
        outputs.insert("subtitle".into(), subtitle);
    }
    if let Some(a) = audio {
        if a != primary {
            outputs.insert("audio".into(), a);
        }
    }

    Ok(InvokeResult {
        primary_output: primary,
        outputs,
    })
}

fn run_preprocess(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-preprocess")?;
    let mut args = vec!["run".into(), "-q".into()];
    args.push("-i".into());
    args.push(req.input.display().to_string());

    if let Some(t) = req.options.get("provider").and_then(ArgValue::as_string) {
        args.push("--provider".into());
        args.push(t);
    } else {
        args.push("--provider".into());
        args.push("stub".into());
    }

    if let Some(chain) = req.options.get("chain").and_then(ArgValue::as_string) {
        args.push("--chain".into());
        args.push(chain);
    } else if let Some(list) = req.options.get("filters").and_then(ArgValue::as_list) {
        let yaml = filters_list_to_yaml(list)?;
        let path = env::temp_dir().join(format!(
            "vd-preprocess-filters-{}-{}.yaml",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::write(&path, yaml).map_err(|e| ExecError::Step(e.to_string()))?;
        args.push("--chain".into());
        args.push(path.display().to_string());
    } else {
        return Err(ExecError::Step(
            "preprocess requires options.filters or options.chain".into(),
        ));
    }

    // Always pass an explicit output path so binder and CLI agree.
    // Default: `{input_parent}/.voxdecoder/work/` (keeps source folder clean).
    let primary = infer_preprocess_output(req);
    if let Some(mut reused) = reuse_existing(req, &primary) {
        if let Some(tm) = timemap_beside(&primary) {
            reused.outputs.insert("timemap".into(), tm);
        }
        return Ok(reused);
    }
    args.push("-o".into());
    args.push(primary.display().to_string());
    push_overwrite(&mut args, req, &primary);

    run_cmd(&bin, &args, req, Some(&primary))?;

    let mut outputs = BTreeMap::new();
    if let Some(tm) = timemap_beside(&primary) {
        outputs.insert("timemap".into(), tm);
    }

    Ok(InvokeResult {
        primary_output: primary,
        outputs,
    })
}

fn timemap_beside(primary: &Path) -> Option<PathBuf> {
    let stem = primary
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prepared");
    let parent = primary.parent().unwrap_or_else(|| Path::new("."));
    let timemap = parent.join(format!("{stem}.timemap.json"));
    timemap.is_file().then_some(timemap)
}

fn infer_preprocess_output(req: &InvokeRequest) -> PathBuf {
    if let Some(o) = &req.output {
        return o.clone();
    }
    let stem = req
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prepared");
    // Video / extract-audio always yields WAV — keep container ext only for pure audio copy-through.
    let ext = if filters_include_extract_audio(req) || is_video_path(&req.input) {
        "wav"
    } else {
        req.input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("wav")
    };
    let name = format!("{stem}.prepared.{ext}");
    if let Some(d) = &req.output_dir {
        return d.join(name);
    }
    default_work_dir(&req.input).join(name)
}

fn filters_include_extract_audio(req: &InvokeRequest) -> bool {
    let Some(list) = req.options.get("filters").and_then(ArgValue::as_list) else {
        return false;
    };
    list.iter().any(|f| {
        f.as_map()
            .and_then(|m| m.get("type").or_else(|| m.get("operation")))
            .and_then(ArgValue::as_string)
            .as_deref()
            == Some("extract-audio")
    })
}

/// `{input_parent}/.voxdecoder/work` — Job intermediates next to the source tree.
fn default_work_dir(input: &Path) -> PathBuf {
    vd_artifact::paths::work_dir_for_input(input)
}

fn explicit_overwrite(req: &InvokeRequest) -> bool {
    req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true)
}

fn same_output_as_input(out: &Path, input: &Path) -> bool {
    if out == input {
        return true;
    }
    match (out.canonicalize(), input.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Idempotent resume: reuse an existing primary artifact instead of regenerating.
///
/// Does **not** reuse when:
/// - `overwrite: true` (caller wants a fresh write), or
/// - output path equals input (fix chain shares `{stem}.fixed.{ext}` — must rewrite in place).
fn reuse_existing(req: &InvokeRequest, primary: &Path) -> Option<InvokeResult> {
    if !primary.is_file() || explicit_overwrite(req) || same_output_as_input(primary, &req.input) {
        return None;
    }
    Some(InvokeResult {
        primary_output: primary.to_path_buf(),
        outputs: BTreeMap::new(),
    })
}

/// Pass `--overwrite` only when explicit, or when rewriting the same path as input (fix chain).
fn want_overwrite(req: &InvokeRequest, out: &Path) -> bool {
    explicit_overwrite(req) || same_output_as_input(out, &req.input)
}

fn push_overwrite(args: &mut Vec<String>, req: &InvokeRequest, out: &Path) {
    if want_overwrite(req, out) {
        args.push("--overwrite".into());
    }
}

fn progress_env(req: &InvokeRequest) -> Vec<(String, String)> {
    let mut env = Vec::new();
    if let Some(path) = &req.progress_snapshot {
        env.push(("VD_PROGRESS_SNAPSHOT".into(), path.display().to_string()));
    }
    if let Some(base) = req.progress_step_base {
        env.push(("VD_PROGRESS_STEP_BASE".into(), base.to_string()));
    }
    if let Some(span) = req.progress_step_span {
        env.push(("VD_PROGRESS_STEP_SPAN".into(), span.to_string()));
    }
    env
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn filters_list_to_yaml(list: &[ArgValue]) -> Result<String, ExecError> {
    let mut filters = Vec::new();
    for item in list {
        let Some(map) = item.as_map() else {
            return Err(ExecError::Step(
                "preprocess filters entries must be maps".into(),
            ));
        };
        let mut m = serde_yaml::Mapping::new();
        for (k, v) in map {
            m.insert(serde_yaml::Value::String(k.clone()), arg_value_to_yaml(v)?);
        }
        filters.push(serde_yaml::Value::Mapping(m));
    }
    let mut root = serde_yaml::Mapping::new();
    root.insert(
        serde_yaml::Value::String("filters".into()),
        serde_yaml::Value::Sequence(filters),
    );
    serde_yaml::to_string(&serde_yaml::Value::Mapping(root))
        .map_err(|e| ExecError::Step(e.to_string()))
}

fn arg_value_to_yaml(v: &ArgValue) -> Result<serde_yaml::Value, ExecError> {
    Ok(match v {
        ArgValue::Bool(b) => serde_yaml::Value::Bool(*b),
        ArgValue::Number(n) => serde_yaml::Value::from(*n),
        ArgValue::String(s) => serde_yaml::Value::String(s.clone()),
        ArgValue::Strings(ss) => serde_yaml::Value::Sequence(
            ss.iter()
                .map(|s| serde_yaml::Value::String(s.clone()))
                .collect(),
        ),
        ArgValue::List(items) => {
            let mut seq = Vec::new();
            for i in items {
                seq.push(arg_value_to_yaml(i)?);
            }
            serde_yaml::Value::Sequence(seq)
        }
        ArgValue::Map(map) => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in map {
                m.insert(serde_yaml::Value::String(k.clone()), arg_value_to_yaml(v)?);
            }
            serde_yaml::Value::Mapping(m)
        }
    })
}

fn run_postprocess(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-postprocess")?;
    let mut args = vec!["run".into(), "-q".into()];

    // Named inputs from options.inputs map; fallback: primary req.input as `input`.
    let mut have_named = false;
    if let Some(map) = req.options.get("inputs").and_then(ArgValue::as_map) {
        for (name, v) in map {
            if let Some(path) = v.as_string() {
                have_named = true;
                args.push("--input".into());
                args.push(format!("{name}={path}"));
            }
        }
    }
    if !have_named {
        args.push("--input".into());
        args.push(format!("input={}", req.input.display()));
    }

    match req.options.get("recipes") {
        Some(ArgValue::Strings(rs)) => {
            for r in rs {
                args.push("--recipe".into());
                args.push(r.clone());
            }
        }
        Some(ArgValue::String(r)) => {
            args.push("--recipe".into());
            args.push(r.clone());
        }
        Some(ArgValue::Map(m)) => {
            for v in m.values() {
                if let Some(r) = v.as_string() {
                    args.push("--recipe".into());
                    args.push(r);
                }
            }
        }
        _ => {
            return Err(ExecError::Step(
                "postprocess requires options.recipes".into(),
            ));
        }
    }

    if let Some(map) = req
        .options
        .get("runner")
        .or_else(|| req.options.get("provider"))
        .and_then(ArgValue::as_map)
    {
        if let Some(t) = map.get("type").and_then(ArgValue::as_string) {
            args.push("--runner".into());
            args.push(t);
        }
        if let Some(m) = map.get("model").and_then(ArgValue::as_string) {
            args.push("-m".into());
            args.push(m);
        }
    } else if let Some(t) = req
        .options
        .get("runner")
        .or_else(|| req.options.get("provider"))
        .and_then(ArgValue::as_string)
    {
        args.push("--runner".into());
        args.push(t);
    } else {
        args.push("--runner".into());
        args.push("stub".into());
    }

    if let Some(map) = req.options.get("variables").and_then(ArgValue::as_map) {
        for (k, v) in map {
            if let Some(val) = v.as_string() {
                args.push("--var".into());
                args.push(format!("{k}={val}"));
            }
        }
    }

    if let Some(d) = &req.output_dir {
        args.push("-d".into());
        args.push(d.display().to_string());
    } else if let Some(o) = &req.output {
        if let Some(parent) = o.parent() {
            args.push("-d".into());
            args.push(parent.display().to_string());
        }
    }

    let primary = req.output.clone().unwrap_or_else(|| {
        req.output_dir
            .clone()
            .unwrap_or_else(|| req.working_dir.clone())
    });
    if primary.is_file() {
        if let Some(reused) = reuse_existing(req, &primary) {
            return Ok(reused);
        }
    }
    push_overwrite(&mut args, req, &primary);

    run_cmd(
        &bin,
        &args,
        req,
        primary.is_file().then_some(primary.as_path()),
    )?;
    Ok(InvokeResult {
        primary_output: primary,
        outputs: BTreeMap::new(),
    })
}

fn run_meeting_merge(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let stem = req
        .options
        .get("artifact_stem")
        .and_then(ArgValue::as_string)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "meeting".into());
    let json_name = format!("{stem}.json");
    let out = req.output.clone().unwrap_or_else(|| {
        req.output_dir
            .as_ref()
            .map_or_else(|| req.working_dir.join(&json_name), |d| d.join(&json_name))
    });
    // If planner passed a bare filename as output, resolve under working/output dir.
    let out = if out.is_relative() && out.components().count() == 1 {
        req.output_dir
            .as_ref()
            .unwrap_or(&req.working_dir)
            .join(out.file_name().unwrap_or_default())
    } else {
        out
    };
    if let Some(reused) = reuse_existing(req, &out) {
        let md_path = meeting_md_path(&out);
        if md_path.is_file() {
            let mut r = reused;
            r.outputs.insert("markdown".into(), md_path);
            return Ok(r);
        }
        // JSON present but markdown missing — regenerate both below.
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ExecError::Step(format!("meeting-merge mkdir: {e}")))?;
    }

    let alignment = req
        .options
        .get("alignment")
        .cloned()
        .unwrap_or(ArgValue::Map(BTreeMap::new()));
    let reference = alignment
        .as_map()
        .and_then(|m| m.get("reference"))
        .and_then(ArgValue::as_string)
        .unwrap_or_else(|| "auto".into());

    let mix_path = req
        .options
        .get("mix")
        .and_then(ArgValue::as_string)
        .map(PathBuf::from);
    let timeline_path = req
        .options
        .get("timeline")
        .and_then(ArgValue::as_string)
        .map(PathBuf::from);

    let mut text_paths: Vec<(String, PathBuf)> = Vec::new();
    if let Some(map) = req.options.get("text_paths").and_then(ArgValue::as_map) {
        for (id, v) in map {
            if let Some(p) = v.as_string() {
                text_paths.push((speaker_from_text_id(&id), PathBuf::from(p)));
            }
        }
    }
    if text_paths.is_empty() {
        // Fallback: primary input is a transcript.
        text_paths.push(("speaker".into(), req.input.clone()));
    }

    // id → display name from meeting.participants.known (avoid listing both id and name).
    let id_to_name = known_speaker_names(&req.options);
    let labels = speaker_labels_map(&req.options);
    let display = |id: &str| -> String {
        let known = id_to_name.get(id);
        let label = labels.get(id);
        match (known, label) {
            // Prefer Cyrillic (etc.) label over a Latinized known[].name / slug.
            (Some(k), Some(l))
                if !has_non_ascii_letter(k) && has_non_ascii_letter(l) =>
            {
                l.clone()
            }
            (Some(k), _) => k.clone(),
            (None, Some(l)) => l.clone(),
            (None, None) => id.to_string(),
        }
    };

    let mix_duration = mix_path.as_ref().and_then(|p| probe_duration_sec(p));
    let timeline = timeline_path
        .as_ref()
        .and_then(|p| load_speaker_timeline(p).ok());

    let mix_text_ids: std::collections::HashSet<String> = req
        .options
        .get("mix_text_ids")
        .and_then(ArgValue::as_strings)
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();

    let mut participant_turns = Vec::new();
    let mut mix_turns = Vec::new();
    let mut participant_ids: Vec<String> = Vec::new();
    let mut cursor = 0.0_f64;
    for (speaker_id, path) in &text_paths {
        // Reconstruct artifact id used in mix_text_ids (branch_id + ".text").
        let text_id = format!("{speaker_id}.text");
        let is_mix = mix_text_ids.contains(&text_id)
            || mix_text_ids.contains(speaker_id)
            || speaker_id.eq_ignore_ascii_case("room");
        // Mix residual must never keep display name "room" — attribute later.
        let speaker = if is_mix {
            speaker_id.clone()
        } else {
            display(speaker_id)
        };
        if !is_mix {
            let name = speaker.clone();
            if !participant_ids.iter().any(|p| p == &name) {
                participant_ids.push(name);
            }
        }
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let mut loaded = Vec::new();
        if let Some(segs) = load_transcript_segments(path) {
            for seg in segs {
                let mut end = seg.end_sec;
                if let Some(max) = mix_duration {
                    end = end.min(max);
                }
                loaded.push(crate::meeting_artifact::MeetingTurn {
                    speaker: speaker.clone(),
                    start_sec: seg.start_sec,
                    end_sec: end.max(seg.start_sec),
                    text: seg.text,
                });
            }
        } else {
            let dur = probe_duration_beside_transcript(path)
                .unwrap_or(1.0)
                .max(0.1);
            let mut end = cursor + dur;
            if let Some(max) = mix_duration {
                end = end.min(max).max(cursor);
            }
            loaded.push(crate::meeting_artifact::MeetingTurn {
                speaker: speaker.clone(),
                start_sec: cursor,
                end_sec: end,
                text,
            });
            cursor = end;
        }
        if is_mix {
            mix_turns.extend(loaded);
        } else {
            participant_turns.extend(loaded);
        }
    }

    // ADR 0016: keep participant turns; mix residual = mix − covered-by-participant.
    let mut mix_residual = subtract_mix_covered_by_participants(&mix_turns, &participant_turns);
    attribute_mix_residual(
        &mut mix_residual,
        timeline.as_ref(),
        &participant_turns,
        &participant_ids,
    );
    scrub_mix_branch_labels(&mut mix_residual, &participant_turns, &participant_ids);
    let mut turns = participant_turns;
    turns.extend(mix_residual);
    turns.sort_by(|a, b| {
        a.start_sec
            .partial_cmp(&b.start_sec)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.speaker.cmp(&b.speaker))
    });

    // Roster = participant display names only (never the mix/room branch id).
    let participants: Vec<String> = {
        let mut seen = BTreeMap::new();
        let mut ordered = Vec::new();
        for name in &participant_ids {
            if seen.insert(name.clone(), ()).is_none() {
                ordered.push(name.clone());
            }
        }
        // Include residual-attributed names that weren't on a text track.
        for t in &turns {
            if seen.insert(t.speaker.clone(), ()).is_none()
                && !is_mix_branch_label(&t.speaker)
            {
                ordered.push(t.speaker.clone());
            }
        }
        ordered
    };

    let effective_reference =
        if timeline.is_some() && (reference == "timeline" || reference == "auto") {
            "timeline"
        } else if mix_path.is_some() && (reference == "mix" || reference == "auto") {
            "mix"
        } else {
            "none"
        };

    let artifact = crate::meeting_artifact::MeetingArtifact {
        version: 1,
        title: None,
        participants: participants.clone(),
        turns,
        timeline,
    };

    let body = serde_json::json!({
        "version": artifact.version,
        "artifact_type": "meeting",
        "title": artifact.title,
        "participants": artifact.participants,
        "turns": artifact.turns,
        "timeline": artifact.timeline,
        "alignment": {
            "mode": alignment.as_map().and_then(|m| m.get("mode")).and_then(ArgValue::as_string).unwrap_or_else(|| "longest".into()),
            "reference": effective_reference,
            "mix": mix_path.as_ref().map(|p| p.display().to_string()),
            "mix_duration_sec": mix_duration,
            "requested_reference": reference,
        },
    });
    let text = serde_json::to_string_pretty(&body)
        .map_err(|e| ExecError::Step(format!("meeting-merge json: {e}")))?;
    std::fs::write(&out, text).map_err(|e| ExecError::Step(format!("meeting-merge write: {e}")))?;

    let md_path = meeting_md_path(&out);
    let md_body = format_meeting_markdown(&artifact.turns);
    std::fs::write(&md_path, md_body)
        .map_err(|e| ExecError::Step(format!("meeting-merge markdown: {e}")))?;

    let mut outputs = BTreeMap::new();
    outputs.insert("markdown".into(), md_path);

    Ok(InvokeResult {
        primary_output: out,
        outputs,
    })
}

fn meeting_md_path(json_out: &Path) -> PathBuf {
    let parent = json_out.parent().unwrap_or_else(|| Path::new("."));
    let stem = json_out
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("meeting");
    parent.join(format!("{stem}.md"))
}

fn known_speaker_names(options: &BTreeMap<String, ArgValue>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(known) = options
        .get("participants")
        .and_then(ArgValue::as_map)
        .and_then(|m| m.get("known"))
        .and_then(ArgValue::as_map)
    else {
        return map;
    };
    for (id, v) in known {
        let name = v
            .as_map()
            .and_then(|m| m.get("name"))
            .and_then(ArgValue::as_string)
            .unwrap_or_else(|| id.clone());
        map.insert(id.clone(), name);
    }
    map
}

/// Planner-provided original-script labels keyed by branch id (`игорь` → `Игорь`).
fn speaker_labels_map(options: &BTreeMap<String, ArgValue>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(labels) = options.get("speaker_labels").and_then(ArgValue::as_map) else {
        return map;
    };
    for (id, v) in labels {
        if let Some(name) = v.as_string() {
            if !name.is_empty() {
                map.insert(id.clone(), name);
            }
        }
    }
    map
}

fn has_non_ascii_letter(s: &str) -> bool {
    s.chars().any(|c| c.is_alphabetic() && !c.is_ascii())
}

/// Human-readable meeting transcript.
/// Speaker header (`**Name**`) only when the speaker changes; consecutive
/// same-speaker turns are blank-line-separated paragraphs under one header.
/// Use `**Name**` (not `[Name]`) so Markdown preview does not treat the label as a link.
fn format_meeting_markdown(turns: &[crate::meeting_artifact::MeetingTurn]) -> String {
    let mut out = String::new();
    let mut last_speaker: Option<&str> = None;
    for turn in turns {
        let text = turn.text.trim();
        if text.is_empty() {
            continue;
        }
        let speaker_changed = last_speaker != Some(turn.speaker.as_str());
        if speaker_changed {
            out.push_str("**");
            out.push_str(&turn.speaker);
            out.push_str("**\n");
            last_speaker = Some(turn.speaker.as_str());
        }
        out.push_str(text);
        out.push_str("\n\n");
    }
    out
}

fn speaker_from_text_id(id: &str) -> String {
    id.strip_suffix(".text")
        .or_else(|| id.strip_suffix(".fixed"))
        .unwrap_or(id)
        .to_string()
}

#[derive(Debug)]
struct SegTurn {
    start_sec: f64,
    end_sec: f64,
    text: String,
}

fn load_transcript_segments(transcript: &Path) -> Option<Vec<SegTurn>> {
    let stem = transcript.file_stem()?.to_str()?;
    // meeting.fixed.txt → look for meeting.segments.json / meeting.fixed.segments.json
    let parent = transcript.parent()?;
    let candidates = [
        parent.join(format!("{stem}.segments.json")),
        parent.join(
            stem.strip_suffix(".fixed")
                .map(|s| format!("{s}.segments.json"))
                .unwrap_or_default(),
        ),
        parent.join(format!("{}.segments.json", stem.trim_end_matches(".fixed"))),
    ];
    let timemap = load_timemap_beside_transcript(transcript);
    for c in candidates {
        if c.as_os_str().is_empty() || !c.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&c).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let segs = v.get("segments")?.as_array()?;
        let mut raw_segs: Vec<(f64, f64, String)> = Vec::new();
        for s in segs {
            let start = s
                .get("start")
                .and_then(|x| x.as_f64())
                .or_else(|| s.get("start_sec").and_then(|x| x.as_f64()))?;
            let end = s
                .get("end")
                .and_then(|x| x.as_f64())
                .or_else(|| s.get("end_sec").and_then(|x| x.as_f64()))?;
            let text = s
                .get("text")
                .or_else(|| s.get("Caption"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if text.is_empty() {
                continue;
            }
            raw_segs.push((start, end, text));
        }
        if raw_segs.is_empty() {
            continue;
        }
        // Executor already remaps ASR sidecars processed→original (ADR 0001 §6).
        // Remap here only when timestamps still sit on the processed clock —
        // otherwise double-apply stretches times by (original/processed)^2
        // (e.g. 5380s → ~37120s) and breaks mix subtract / attribution.
        let needs_remap = timemap.as_ref().is_some_and(|tm| {
            tm.timestamps_on_processed_clock(raw_segs.iter().map(|(_, end, _)| *end))
        });
        let out: Vec<SegTurn> = raw_segs
            .into_iter()
            .map(|(start, end, text)| {
                let (start_sec, end_sec) = if needs_remap {
                    timemap
                        .as_ref()
                        .map(|tm| tm.remap_interval(start, end))
                        .unwrap_or((start, end))
                } else {
                    (start, end)
                };
                SegTurn {
                    start_sec,
                    end_sec,
                    text,
                }
            })
            .collect();
        return Some(out);
    }
    None
}

fn load_timemap_beside_transcript(transcript: &Path) -> Option<vd_artifact::TimeMap> {
    let stem = transcript.file_stem()?.to_str()?;
    let parent = transcript.parent()?;
    let bases = [
        stem.strip_suffix(".fixed").unwrap_or(stem).to_string(),
        stem.trim_end_matches(".fixed").to_string(),
        stem.to_string(),
    ];
    for base in bases {
        if base.is_empty() {
            continue;
        }
        let path = parent.join(format!("{base}.timemap.json"));
        if !path.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&path).ok()?;
        if let Ok(tm) = serde_json::from_str::<vd_artifact::TimeMap>(&raw) {
            return Some(tm);
        }
    }
    None
}

/// Best-effort duration of prepared media next to a transcript path.
fn probe_duration_beside_transcript(transcript: &Path) -> Option<f64> {
    let stem = transcript.file_stem()?.to_str()?;
    let parent = transcript.parent()?;
    let base = stem.strip_suffix(".fixed").unwrap_or(stem);
    let base = base.trim_end_matches(".fixed");
    for ext in ["mp3", "wav", "m4a", "ogg", "flac", "mp4"] {
        let media = parent.join(format!("{base}.{ext}"));
        if media.is_file() {
            if let Some(d) = probe_duration_sec(&media) {
                return Some(d);
            }
        }
    }
    None
}

fn load_speaker_timeline(
    path: &Path,
) -> Result<crate::meeting_artifact::SpeakerTimeline, ExecError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| ExecError::Step(format!("read timeline: {e}")))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| ExecError::Step(format!("parse timeline: {e}")))?;
    // Prefer vd-diarize shape (`segments` with start/end); fall back to pipeline shape.
    if let Some(segments) = value.get("segments").and_then(|v| v.as_array()) {
        let speakers = segments
            .iter()
            .filter_map(|seg| {
                let speaker = seg.get("speaker")?.as_str()?.to_string();
                let start_sec = seg.get("start")?.as_f64()?;
                let end_sec = seg.get("end")?.as_f64()?;
                let confidence = seg.get("confidence").and_then(|v| v.as_f64());
                Some(crate::meeting_artifact::SpeakerSegment {
                    speaker,
                    start_sec,
                    end_sec,
                    confidence,
                })
            })
            .collect();
        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        let overlaps = value
            .get("overlaps")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|o| {
                        Some(crate::meeting_artifact::OverlapRegion {
                            start_sec: o.get("start")?.as_f64()?,
                            end_sec: o.get("end")?.as_f64()?,
                            speakers: o
                                .get("speakers")
                                .and_then(|s| s.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|x| x.as_str().map(str::to_string))
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Ok(crate::meeting_artifact::SpeakerTimeline {
            version,
            speakers,
            overlaps,
        });
    }
    serde_json::from_value(value).map_err(|e| ExecError::Step(format!("parse timeline: {e}")))
}

/// ADR 0016: drop mix turns that are time-overlapping and lexically near a participant turn.
fn subtract_mix_covered_by_participants(
    mix: &[crate::meeting_artifact::MeetingTurn],
    participants: &[crate::meeting_artifact::MeetingTurn],
) -> Vec<crate::meeting_artifact::MeetingTurn> {
    // Slightly below exact-match ASR variance (РО vs СРО, etc.).
    const SIM_THRESHOLD: f64 = 0.80;
    mix.iter()
        .filter(|m| {
            let m_norm = normalize_compare(&m.text);
            if m_norm.is_empty() {
                return false;
            }
            !participants.iter().any(|p| {
                let time_overlap = m.start_sec < p.end_sec && p.start_sec < m.end_sec;
                if !time_overlap {
                    return false;
                }
                let p_norm = normalize_compare(&p.text);
                if p_norm.is_empty() {
                    return false;
                }
                if p_norm == m_norm {
                    return true;
                }
                vd_text::similarity::asr_near_duplicate_ratio(&p_norm, &m_norm) >= SIM_THRESHOLD
            })
        })
        .cloned()
        .collect()
}

fn is_mix_branch_label(label: &str) -> bool {
    label.eq_ignore_ascii_case("room") || label.eq_ignore_ascii_case("merged")
}

/// Relabel mix residual: never keep `room` / mix branch id in the artifact.
/// Prefer failed-track fallback when one participant ASR is near-empty; else
/// diarize-timeline → participant correlation; else weakest / sole / `Unknown`.
fn attribute_mix_residual(
    residual: &mut [crate::meeting_artifact::MeetingTurn],
    timeline: Option<&crate::meeting_artifact::SpeakerTimeline>,
    participant_turns: &[crate::meeting_artifact::MeetingTurn],
    participant_names: &[String],
) {
    if residual.is_empty() {
        return;
    }

    let fallback = residual_fallback_name(participant_turns, participant_names);

    // Failed / near-empty track: their speech lives only on the mix. Stub or
    // sparse diarize would otherwise map every residual window onto the strong
    // track (the only one with overlap evidence) and steal the failed speaker's
    // words. Skip diarize correlation in that case.
    if failed_participant_track(participant_turns, participant_names).is_some() {
        for turn in residual.iter_mut() {
            turn.speaker = fallback.clone();
        }
        return;
    }

    let diarize_map = timeline
        .map(|tl| correlate_diarize_to_participants(tl, participant_turns, participant_names))
        .unwrap_or_default();

    for turn in residual.iter_mut() {
        if let Some(tl) = timeline {
            if let Some(ds) = active_diarize_speaker(tl, turn.start_sec, turn.end_sec) {
                if let Some(name) = diarize_map.get(&ds) {
                    turn.speaker = name.clone();
                    continue;
                }
            }
        }
        turn.speaker = fallback.clone();
    }
}

/// Last-line scrub: any leftover `room`/`merged` label → fallback name.
fn scrub_mix_branch_labels(
    turns: &mut [crate::meeting_artifact::MeetingTurn],
    participant_turns: &[crate::meeting_artifact::MeetingTurn],
    participant_names: &[String],
) {
    let fallback = residual_fallback_name(participant_turns, participant_names);
    for turn in turns.iter_mut() {
        if is_mix_branch_label(&turn.speaker) {
            turn.speaker = fallback.clone();
        }
    }
}

fn residual_fallback_name(
    participant_turns: &[crate::meeting_artifact::MeetingTurn],
    participant_names: &[String],
) -> String {
    if participant_names.len() == 1 {
        return participant_names[0].clone();
    }
    failed_participant_track(participant_turns, participant_names)
        .or_else(|| weakest_participant(participant_turns, participant_names))
        .unwrap_or_else(|| "Unknown".into())
}

/// Participant whose ASR coverage is near-empty while another has real content.
/// Typical when preprocess/ASR killed one track and speech survives only on mix.
fn failed_participant_track(
    turns: &[crate::meeting_artifact::MeetingTurn],
    names: &[String],
) -> Option<String> {
    if names.len() < 2 {
        return None;
    }
    let mut counts: Vec<(usize, &str)> = Vec::with_capacity(names.len());
    for name in names {
        let chars: usize = turns
            .iter()
            .filter(|t| &t.speaker == name)
            .map(|t| t.text.chars().count())
            .sum();
        counts.push((chars, name.as_str()));
    }
    let max = counts.iter().map(|(c, _)| *c).max()?;
    let (min_c, min_n) = counts.iter().copied().min_by_key(|(c, _)| *c)?;
    // Near-empty vs real content (e.g. "Д." vs multi-kchar track).
    if min_c <= 32 && max >= 200 && min_c.saturating_mul(20) < max {
        Some(min_n.to_string())
    } else {
        None
    }
}

fn weakest_participant(
    turns: &[crate::meeting_artifact::MeetingTurn],
    names: &[String],
) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let mut best: Option<(usize, &str)> = None;
    for name in names {
        let chars: usize = turns
            .iter()
            .filter(|t| &t.speaker == name)
            .map(|t| t.text.chars().count())
            .sum();
        match best {
            None => best = Some((chars, name.as_str())),
            Some((c, _)) if chars < c => best = Some((chars, name.as_str())),
            _ => {}
        }
    }
    best.map(|(_, n)| n.to_string())
}

fn active_diarize_speaker(
    timeline: &crate::meeting_artifact::SpeakerTimeline,
    start: f64,
    end: f64,
) -> Option<String> {
    let mut best: Option<(f64, String)> = None;
    for seg in &timeline.speakers {
        let o0 = start.max(seg.start_sec);
        let o1 = end.min(seg.end_sec);
        let dur = (o1 - o0).max(0.0);
        if dur <= 0.0 {
            continue;
        }
        if best.as_ref().map(|(d, _)| dur > *d).unwrap_or(true) {
            best = Some((dur, seg.speaker.clone()));
        }
    }
    best.map(|(_, s)| s)
}

/// Map diarize cluster ids → participant display names by max time-overlap.
fn correlate_diarize_to_participants(
    timeline: &crate::meeting_artifact::SpeakerTimeline,
    participant_turns: &[crate::meeting_artifact::MeetingTurn],
    participant_names: &[String],
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if participant_names.is_empty() || timeline.speakers.is_empty() {
        return out;
    }
    let mut diarize_ids: Vec<String> = timeline
        .speakers
        .iter()
        .map(|s| s.speaker.clone())
        .collect();
    diarize_ids.sort();
    diarize_ids.dedup();

    for did in diarize_ids {
        let mut best: Option<(f64, String)> = None;
        for pname in participant_names {
            let mut overlap = 0.0_f64;
            for dseg in timeline.speakers.iter().filter(|s| s.speaker == did) {
                for pt in participant_turns.iter().filter(|t| &t.speaker == pname) {
                    let o0 = dseg.start_sec.max(pt.start_sec);
                    let o1 = dseg.end_sec.min(pt.end_sec);
                    overlap += (o1 - o0).max(0.0);
                }
            }
            if best.as_ref().map(|(d, _)| overlap > *d).unwrap_or(true) {
                best = Some((overlap, pname.clone()));
            }
        }
        if let Some((ov, name)) = best {
            // Require some evidence; otherwise leave unmapped.
            if ov > 0.5 {
                out.insert(did, name);
            }
        }
    }
    out
}

fn normalize_compare(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = false;
    for c in text.trim().chars() {
        if c.is_whitespace() {
            if !last_was_space && !out.is_empty() {
                out.push(' ');
                last_was_space = true;
            }
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            last_was_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

fn probe_duration_sec(path: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse().ok()
}

fn run_diarize(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-diarize")?;
    let primary = infer_diarize_output(req);
    if let Some(reused) = reuse_existing(req, &primary) {
        return Ok(reused);
    }
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-q".into(),
        "-o".into(),
        primary.display().to_string(),
    ];

    let (provider, model) = diarize_backend(&req.options);
    if let Some(p) = provider {
        args.push("--backend".into());
        args.push(p);
    }
    if let Some(m) = model {
        args.push("-m".into());
        args.push(m);
    }
    if let Some(d) = req.options.get("device").and_then(ArgValue::as_string) {
        args.push("--device".into());
        args.push(d);
    }
    push_overwrite(&mut args, req, &primary);

    run_cmd(&bin, &args, req, Some(&primary))?;
    Ok(InvokeResult {
        primary_output: primary,
        outputs: BTreeMap::new(),
    })
}

/// Resolve `options.backend.{provider,model}` or flat `provider` / `backend` + `model`.
fn diarize_backend(options: &BTreeMap<String, ArgValue>) -> (Option<String>, Option<String>) {
    if let Some(map) = options.get("backend").and_then(ArgValue::as_map) {
        let provider = map.get("provider").and_then(ArgValue::as_string);
        let model = map.get("model").and_then(ArgValue::as_string);
        return (provider, model);
    }
    let provider = options
        .get("provider")
        .or_else(|| options.get("backend"))
        .and_then(ArgValue::as_string);
    let model = options.get("model").and_then(ArgValue::as_string);
    (provider, model)
}

fn infer_diarize_output(req: &InvokeRequest) -> PathBuf {
    if let Some(o) = &req.output {
        return o.clone();
    }
    let stem = req
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let name = format!("{stem}.diarization.json");
    if let Some(d) = &req.output_dir {
        return d.join(name);
    }
    default_work_dir(&req.input).join(name)
}

fn run_transcribe(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let engine = req
        .options
        .get("engine")
        .and_then(ArgValue::as_string)
        .unwrap_or_else(|| "gigaam".into());
    if engine == "whisper" {
        return Err(ExecError::Reserved(
            "whisper is reserved; vd-whisper is not available yet".into(),
        ));
    }
    if engine != "gigaam" {
        return Err(ExecError::Step(format!(
            "unknown transcribe engine: {engine}"
        )));
    }

    let bin = find_bin("vd-gigaam")?;
    let out = infer_gigaam_output(req);
    if let Some(mut reused) = reuse_existing(req, &out) {
        let seg = segments_sidecar_for(&out);
        if seg.is_file() {
            reused.outputs.insert("segments".into(), seg);
        }
        return Ok(reused);
    }
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-q".into(),
        "-o".into(),
        out.display().to_string(),
    ];
    if let Some(m) = req.options.get("model").and_then(ArgValue::as_string) {
        args.push("-m".into());
        args.push(m);
    }
    if let Some(d) = req.options.get("device").and_then(ArgValue::as_string) {
        args.push("--device".into());
        args.push(d);
    }
    // vd-gigaam exposes --flash only on non-macOS (CUDA) builds.
    #[cfg(not(target_os = "macos"))]
    if req.options.get("flash").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--flash".into());
    }
    push_overwrite(&mut args, req, &out);
    if req.options.get("segments").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--segments".into());
    }
    if req
        .options
        .get("word_timestamps")
        .and_then(ArgValue::as_bool)
        == Some(true)
    {
        args.push("--word-timestamps".into());
    }
    if let Some(fmt) = req.options.get("format").and_then(ArgValue::as_string) {
        args.push("--format".into());
        args.push(fmt);
    }

    run_cmd(&bin, &args, req, Some(&out))?;
    let mut outputs = BTreeMap::new();
    let seg = segments_sidecar_for(&out);
    if seg.is_file() {
        outputs.insert("segments".into(), seg);
    }
    Ok(InvokeResult {
        primary_output: out,
        outputs,
    })
}

fn segments_sidecar_for(main: &Path) -> PathBuf {
    let stem = main.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let parent = main.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}.segments.json"))
}

fn infer_gigaam_output(req: &InvokeRequest) -> PathBuf {
    if let Some(o) = &req.output {
        return o.clone();
    }
    let stem = req
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let name = format!("{stem}.txt");
    if let Some(d) = &req.output_dir {
        return d.join(name);
    }
    default_work_dir(&req.input).join(name)
}

fn run_prepare_context(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-assets")?;
    let out_dir = req
        .output
        .clone()
        .or_else(|| req.context_assets.clone())
        .unwrap_or_else(|| req.working_dir.join(".voxdecoder"));

    // No docs root / nothing convertible → empty assets dir (fix-* still run).
    if !req.input.exists() || !docs_have_sources(&req.input) {
        return ensure_empty_assets(&out_dir);
    }

    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-o".into(),
        out_dir.display().to_string(),
        "-q".into(),
    ];
    if req.options.get("ocr").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--ocr".into());
    }
    if req.options.get("force").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--force".into());
    }
    run_cmd(&bin, &args, req, None)?;
    Ok(InvokeResult {
        primary_output: out_dir,
        outputs: BTreeMap::new(),
    })
}

fn ensure_empty_assets(out_dir: &Path) -> Result<InvokeResult, ExecError> {
    std::fs::create_dir_all(out_dir.join("md")).map_err(|e| ExecError::Step(e.to_string()))?;
    let terms = out_dir.join("terms.yml");
    if !terms.exists() {
        std::fs::write(&terms, "version: 1\nentries: []\nforms: []\n")
            .map_err(|e| ExecError::Step(e.to_string()))?;
    }
    Ok(InvokeResult {
        primary_output: out_dir.to_path_buf(),
        outputs: BTreeMap::new(),
    })
}

fn docs_have_sources(root: &Path) -> bool {
    fn is_source(p: &Path) -> bool {
        matches!(
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref(),
            Some(
                "md" | "markdown"
                    | "txt"
                    | "rst"
                    | "pdf"
                    | "docx"
                    | "doc"
                    | "xlsx"
                    | "xls"
                    | "pptx"
                    | "ppt"
                    | "odt"
                    | "ods",
            )
        )
    }
    if root.is_file() {
        return is_source(root);
    }
    let Ok(walk) = std::fs::read_dir(root) else {
        return false;
    };
    for entry in walk.flatten() {
        let p = entry.path();
        if p.is_file() && is_source(&p) {
            return true;
        }
        if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
            if docs_have_sources(&p) {
                return true;
            }
        }
    }
    false
}

fn run_fix(req: &InvokeRequest, bin_name: &str) -> Result<InvokeResult, ExecError> {
    let bin = find_bin(bin_name)?;
    let primary = infer_fix_output(req);
    if let Some(reused) = reuse_existing(req, &primary) {
        return Ok(reused);
    }
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-q".into(),
        "-o".into(),
        primary.display().to_string(),
    ];
    push_overwrite(&mut args, req, &primary);
    if let Some(lang) = req.options.get("language").and_then(ArgValue::as_string) {
        args.push("-l".into());
        args.push(lang);
    }
    if bin_name == "vd-fix-asr" {
        if let Some(ctx) = &req.context_assets {
            args.push("--context".into());
            args.push(ctx.display().to_string());
        }
    }
    if bin_name == "vd-fix-terms" {
        if let Some(ctx) = &req.context_assets {
            args.push("--terms".into());
            args.push(ctx.display().to_string());
        }
    }
    if bin_name == "vd-fix-layout" {
        if let Some(d) = req.options.get("density").and_then(ArgValue::as_string) {
            args.push("--density".into());
            args.push(d);
        }
        if req.options.get("use_timemap").and_then(ArgValue::as_bool) == Some(false) {
            args.push("--no-timemap".into());
        } else if let Some(tm) = req.options.get("timemap").and_then(ArgValue::as_string) {
            args.push("--timemap".into());
            args.push(tm);
        }
    }
    run_cmd(&bin, &args, req, Some(&primary))?;
    Ok(InvokeResult {
        primary_output: primary,
        outputs: BTreeMap::new(),
    })
}

/// `vd-fix-overlap` has a different CLI shape from the other `vd-fix-*`
/// tools — `-i`/`-o` alone only *report* candidate duplicates, it needs
/// `--apply` to actually remove/trim and write. No `-l/--language` (the
/// detector is language-agnostic).
///
/// Meeting pipeline: merge writes `meeting.json` + `meeting.md`, then this
/// step rewrites JSON in place. Sidecar markdown must be regenerated from the
/// deduped turns (ADR 0016) — otherwise `.md` keeps cross-speaker bleed.
fn run_fix_overlap(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-fix-overlap")?;
    let primary = infer_fix_output(req);
    if let Some(mut reused) = reuse_existing(req, &primary) {
        if let Some(md) = sync_meeting_markdown_from_json(&primary)? {
            reused.outputs.insert("markdown".into(), md);
        }
        return Ok(reused);
    }
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-q".into(),
        "-o".into(),
        primary.display().to_string(),
        "--apply".into(),
    ];
    push_overwrite(&mut args, req, &primary);
    run_cmd(&bin, &args, req, Some(&primary))?;
    let mut outputs = BTreeMap::new();
    if let Some(md) = sync_meeting_markdown_from_json(&primary)? {
        outputs.insert("markdown".into(), md);
    }
    Ok(InvokeResult {
        primary_output: primary,
        outputs,
    })
}

/// Rewrite sibling `*.md` from meeting JSON turns after fix-overlap.
/// Returns `None` when `json_path` is not a meeting turns document.
fn sync_meeting_markdown_from_json(json_path: &Path) -> Result<Option<PathBuf>, ExecError> {
    let Some(turns) = load_meeting_turns_for_md(json_path)? else {
        return Ok(None);
    };
    let md_path = meeting_md_path(json_path);
    let body = format_meeting_markdown(&turns);
    std::fs::write(&md_path, body)
        .map_err(|e| ExecError::Step(format!("fix-overlap markdown sync: {e}")))?;
    Ok(Some(md_path))
}

fn load_meeting_turns_for_md(
    json_path: &Path,
) -> Result<Option<Vec<crate::meeting_artifact::MeetingTurn>>, ExecError> {
    let text = match std::fs::read_to_string(json_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(ExecError::Step(format!(
                "fix-overlap markdown sync read {}: {e}",
                json_path.display()
            )));
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let Some(turns_val) = value.get("turns") else {
        return Ok(None);
    };
    let turns: Vec<crate::meeting_artifact::MeetingTurn> =
        match serde_json::from_value(turns_val.clone()) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
    Ok(Some(turns))
}

fn infer_fix_output(req: &InvokeRequest) -> PathBuf {
    if let Some(o) = &req.output {
        return o.clone();
    }
    let stem = req
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let ext = req
        .input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt");
    // strip prior .fixed
    let stem = stem.strip_suffix(".fixed").unwrap_or(stem);
    let name = format!("{stem}.fixed.{ext}");
    if let Some(d) = &req.output_dir {
        return d.join(name);
    }
    default_work_dir(&req.input).join(name)
}

fn run_cmd(
    bin: &Path,
    args: &[String],
    req: &InvokeRequest,
    expected_out: Option<&Path>,
) -> Result<(), ExecError> {
    let mut cmd = Command::new(bin);
    cmd.args(args).current_dir(&req.working_dir);
    for (k, v) in progress_env(req) {
        cmd.env(k, v);
    }
    let output = cmd
        .output()
        .map_err(|e| ExecError::Step(format!("{}: {e}", bin.display())))?;
    if output.status.success() {
        // Child tools may still print progress/status on stdout when not fully quiet.
        if !output.stdout.is_empty() {
            let _ = std::io::Write::write_all(&mut std::io::stdout(), &output.stdout);
        }
        return Ok(());
    }
    let code = output.status.code().unwrap_or(1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = stderr.trim();
    let detail = if detail.is_empty() {
        stdout.trim()
    } else {
        detail
    };
    // Soft resume: child still reported AlreadyExists but the artifact is on disk.
    if detail.contains("output already exists") {
        if let Some(p) = expected_out {
            if p.is_file() && !explicit_overwrite(req) && !same_output_as_input(p, &req.input) {
                return Ok(());
            }
        }
    }
    let detail = compact_step_error(detail);
    if detail.is_empty() {
        Err(ExecError::Step(format!("{} exited {code}", bin.display())))
    } else {
        Err(ExecError::Step(format!(
            "{} exited {code}: {detail}",
            bin.display()
        )))
    }
}

/// Keep child failure text short enough for MCP/job.status JSON.
fn compact_step_error(raw: &str) -> String {
    const MAX: usize = 1500;
    let cleaned = raw.replace('\r', "\n");
    let mut lines: Vec<&str> = cleaned
        .lines()
        .map(str::trim)
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("ffmpeg version")
                && !l.starts_with("built with")
                && !l.starts_with("configuration:")
                && !l.starts_with("libav")
                && !l.starts_with("libsw")
                && !l.starts_with("size=")
                && !l.starts_with("frame=")
        })
        .collect();
    if lines.len() > 20 {
        lines = lines.split_off(lines.len() - 20);
    }
    let joined = lines.join("\n");
    if joined.len() <= MAX {
        return joined;
    }
    let start = joined.len().saturating_sub(MAX);
    format!("…{}", joined[start..].trim_start())
}

fn find_bin(name: &str) -> Result<PathBuf, ExecError> {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
            #[cfg(windows)]
            {
                let bat = dir.join(format!("{name}.exe"));
                if bat.is_file() {
                    return Ok(bat);
                }
            }
        }
    }
    Ok(PathBuf::from(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn req(input: &Path, overwrite: bool) -> InvokeRequest {
        let mut options = BTreeMap::new();
        if overwrite {
            options.insert("overwrite".into(), ArgValue::Bool(true));
        }
        InvokeRequest {
            capability: Capability::FixAsr,
            step_id: None,
            working_dir: input.parent().unwrap_or(Path::new(".")).to_path_buf(),
            input: input.to_path_buf(),
            output: None,
            output_dir: None,
            context_assets: None,
            options,
            progress_snapshot: None,
            progress_step_base: None,
            progress_step_span: None,
        }
    }

    #[test]
    fn reuse_existing_skips_when_artifact_present() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("igor.transcript.txt");
        let fixed = dir.path().join("igor.transcript.fixed.txt");
        fs::write(&input, "raw").unwrap();
        fs::write(&fixed, "old fixed").unwrap();

        let r = req(&input, false);
        let reused = reuse_existing(&r, &fixed).expect("should reuse");
        assert_eq!(reused.primary_output, fixed);
        assert!(!want_overwrite(&r, &fixed));
    }

    #[test]
    fn reuse_existing_refuses_in_place_fix_chain() {
        let dir = tempfile::tempdir().unwrap();
        let fixed = dir.path().join("igor.transcript.fixed.txt");
        fs::write(&fixed, "cased").unwrap();

        let r = req(&fixed, false);
        assert!(reuse_existing(&r, &fixed).is_none());
        assert!(want_overwrite(&r, &fixed));
    }

    #[test]
    fn reuse_existing_honors_explicit_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("igor.transcript.txt");
        let fixed = dir.path().join("igor.transcript.fixed.txt");
        fs::write(&input, "raw").unwrap();
        fs::write(&fixed, "old").unwrap();

        let r = req(&input, true);
        assert!(reuse_existing(&r, &fixed).is_none());
        assert!(want_overwrite(&r, &fixed));
    }

    #[test]
    fn meeting_markdown_speaker_blocks() {
        let turns = vec![
            crate::meeting_artifact::MeetingTurn {
                speaker: "Игорь".into(),
                start_sec: 0.0,
                end_sec: 1.0,
                text: "Привет".into(),
            },
            crate::meeting_artifact::MeetingTurn {
                speaker: "Владимир".into(),
                start_sec: 1.0,
                end_sec: 2.0,
                text: "Здравствуй".into(),
            },
        ];
        let md = format_meeting_markdown(&turns);
        assert_eq!(md, "**Игорь**\nПривет\n\n**Владимир**\nЗдравствуй\n\n");
    }

    #[test]
    fn meeting_markdown_collapses_consecutive_same_speaker() {
        let turns = vec![
            crate::meeting_artifact::MeetingTurn {
                speaker: "Игорь".into(),
                start_sec: 0.0,
                end_sec: 1.0,
                text: "Первый".into(),
            },
            crate::meeting_artifact::MeetingTurn {
                speaker: "Игорь".into(),
                start_sec: 1.0,
                end_sec: 2.0,
                text: "Второй".into(),
            },
            crate::meeting_artifact::MeetingTurn {
                speaker: "Владимир".into(),
                start_sec: 2.0,
                end_sec: 3.0,
                text: "Ответ".into(),
            },
            crate::meeting_artifact::MeetingTurn {
                speaker: "Игорь".into(),
                start_sec: 3.0,
                end_sec: 4.0,
                text: "Снова".into(),
            },
        ];
        let md = format_meeting_markdown(&turns);
        assert_eq!(
            md,
            "**Игорь**\nПервый\n\nВторой\n\n**Владимир**\nОтвет\n\n**Игорь**\nСнова\n\n"
        );
    }

    #[test]
    fn sync_meeting_markdown_from_deduped_json() {
        let dir = tempfile::tempdir().unwrap();
        let json = dir.path().join("meeting.json");
        let stale_md = dir.path().join("meeting.md");
        fs::write(
            &json,
            r#"{
              "version": 1,
              "artifact_type": "meeting",
              "turns": [
                {"speaker":"igor","start_sec":0.0,"end_sec":1.0,"text":"hello"},
                {"speaker":"vladimir","start_sec":1.0,"end_sec":2.0,"text":"hi"}
              ]
            }"#,
        )
        .unwrap();
        // Stale pre-dedupe markdown still has the bleed copy.
        fs::write(
            &stale_md,
            "**igor**\nhello\n\n**vladimir**\nhello\n\n**vladimir**\nhi\n\n",
        )
        .unwrap();

        let md = sync_meeting_markdown_from_json(&json).unwrap().unwrap();
        assert_eq!(md, stale_md);
        let body = fs::read_to_string(&md).unwrap();
        assert_eq!(body, "**igor**\nhello\n\n**vladimir**\nhi\n\n");
        assert!(!body.contains("**vladimir**\nhello"));
    }

    #[test]
    fn sync_meeting_markdown_skips_non_meeting_json() {
        let dir = tempfile::tempdir().unwrap();
        let json = dir.path().join("plain.json");
        fs::write(&json, r#"{"segments":[]}"#).unwrap();
        assert!(sync_meeting_markdown_from_json(&json).unwrap().is_none());
        assert!(!dir.path().join("plain.md").exists());
    }

    #[test]
    fn known_names_map_id_to_display() {
        let mut known = BTreeMap::new();
        let mut igor = BTreeMap::new();
        igor.insert("name".into(), ArgValue::String("Игорь".into()));
        known.insert("igor".into(), ArgValue::Map(igor));
        let mut participants = BTreeMap::new();
        participants.insert("known".into(), ArgValue::Map(known));
        let mut options = BTreeMap::new();
        options.insert("participants".into(), ArgValue::Map(participants));
        let map = known_speaker_names(&options);
        assert_eq!(map.get("igor").map(String::as_str), Some("Игорь"));
    }

    #[test]
    fn speaker_labels_preserve_cyrillic_display() {
        let mut labels = BTreeMap::new();
        labels.insert("игорь".into(), ArgValue::String("Игорь".into()));
        let mut options = BTreeMap::new();
        options.insert("speaker_labels".into(), ArgValue::Map(labels));
        let map = speaker_labels_map(&options);
        assert_eq!(map.get("игорь").map(String::as_str), Some("Игорь"));
    }

    #[test]
    fn mix_residual_never_keeps_room_label() {
        let participant_turns = vec![
            crate::meeting_artifact::MeetingTurn {
                speaker: "Владимир".into(),
                start_sec: 0.0,
                end_sec: 10.0,
                text: "привет от владимира длинный текст чтобы покрыть".into(),
            },
            crate::meeting_artifact::MeetingTurn {
                speaker: "Игорь".into(),
                start_sec: 10.0,
                end_sec: 12.0,
                text: "ок".into(),
            },
        ];
        let mut residual = vec![crate::meeting_artifact::MeetingTurn {
            speaker: "room".into(),
            start_sec: 20.0,
            end_sec: 30.0,
            text: "остаток с микса который не покрыт треками".into(),
        }];
        attribute_mix_residual(
            &mut residual,
            None,
            &participant_turns,
            &["Владимир".into(), "Игорь".into()],
        );
        assert_ne!(residual[0].speaker, "room");
        // Weakest track (Игорь) gets residual when timeline absent.
        assert_eq!(residual[0].speaker, "Игорь");
    }

    #[test]
    fn failed_track_residual_ignores_diarize_steal() {
        // Strong track has all diarize overlap evidence; failed track has "Д.".
        // Stub/sparse diarize would map residual → Владимир; must keep Николай.
        let participant_turns = vec![
            crate::meeting_artifact::MeetingTurn {
                speaker: "Владимир".into(),
                start_sec: 0.0,
                end_sec: 60.0,
                text: "а".repeat(400),
            },
            crate::meeting_artifact::MeetingTurn {
                speaker: "Николай".into(),
                start_sec: 5380.0,
                end_sec: 5406.0,
                text: "Д.".into(),
            },
        ];
        let timeline = crate::meeting_artifact::SpeakerTimeline {
            version: 1,
            speakers: vec![
                crate::meeting_artifact::SpeakerSegment {
                    speaker: "S0".into(),
                    start_sec: 0.0,
                    end_sec: 61.0,
                    confidence: Some(1.0),
                },
                crate::meeting_artifact::SpeakerSegment {
                    speaker: "S1".into(),
                    start_sec: 59.0,
                    end_sec: 120.0,
                    confidence: Some(1.0),
                },
            ],
            overlaps: vec![],
        };
        let mut residual = vec![crate::meeting_artifact::MeetingTurn {
            speaker: "room".into(),
            start_sec: 20.0,
            end_sec: 40.0,
            text: "речь коуча только на миксе".into(),
        }];
        attribute_mix_residual(
            &mut residual,
            Some(&timeline),
            &participant_turns,
            &["Владимир".into(), "Николай".into()],
        );
        assert_eq!(residual[0].speaker, "Николай");
        scrub_mix_branch_labels(
            &mut residual,
            &participant_turns,
            &["Владимир".into(), "Николай".into()],
        );
        assert!(!is_mix_branch_label(&residual[0].speaker));
    }

    #[test]
    fn scrub_rewrites_stray_room_label() {
        let mut turns = vec![crate::meeting_artifact::MeetingTurn {
            speaker: "room".into(),
            start_sec: 0.0,
            end_sec: 1.0,
            text: "ещё остаток".into(),
        }];
        let participants = vec![crate::meeting_artifact::MeetingTurn {
            speaker: "Владимир".into(),
            start_sec: 0.0,
            end_sec: 1.0,
            text: "x".repeat(250),
        }];
        scrub_mix_branch_labels(&mut turns, &participants, &["Владимир".into(), "Николай".into()]);
        assert_eq!(turns[0].speaker, "Николай");
    }

    #[test]
    fn subtract_drops_near_duplicate_mix_bleed() {
        let participants = vec![crate::meeting_artifact::MeetingTurn {
            speaker: "Владимир".into(),
            start_sec: 0.0,
            end_sec: 20.0,
            text: "Продукта зависят. И я с тобой согласен. Это круто, когда есть девопсы".into(),
        }];
        let mix = vec![crate::meeting_artifact::MeetingTurn {
            speaker: "room".into(),
            start_sec: 5.0,
            end_sec: 15.0,
            text: "Продукта зависят. я с тобой согласен. Это круто, когда есть девопсы".into(),
        }];
        let left = subtract_mix_covered_by_participants(&mix, &participants);
        assert!(left.is_empty(), "near-duplicate mix bleed must drop");
    }
}
