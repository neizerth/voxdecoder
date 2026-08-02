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
            Capability::FixTerms => run_fix(req, "vd-fix-terms"),
            Capability::FixLayout => run_fix(req, "vd-fix-layout"),
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
        .or_else(|| req.output.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)))
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

    run_cmd(&bin, &args, &req.working_dir)?;

    let audio = ["audio.m4a", "audio.wav", "audio.mp3", "audio.ogg", "audio.opus"]
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
    args.push("-o".into());
    args.push(primary.display().to_string());
    if req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--overwrite".into());
    }

    run_cmd(&bin, &args, &req.working_dir)?;

    let mut outputs = BTreeMap::new();
    let timemap = {
        let stem = primary
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("prepared");
        let parent = primary.parent().unwrap_or_else(|| Path::new("."));
        parent.join(format!("{stem}.timemap.json"))
    };
    if timemap.is_file() {
        outputs.insert("timemap".into(), timemap);
    }

    Ok(InvokeResult {
        primary_output: primary,
        outputs,
    })
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
            m.insert(
                serde_yaml::Value::String(k.clone()),
                arg_value_to_yaml(v)?,
            );
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
                m.insert(
                    serde_yaml::Value::String(k.clone()),
                    arg_value_to_yaml(v)?,
                );
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

    if req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--overwrite".into());
    }

    run_cmd(&bin, &args, &req.working_dir)?;

    // Primary output: first recipe's first declared file is unknown here; use output_dir or working_dir.
    let primary = req.output.clone().unwrap_or_else(|| {
        req.output_dir
            .clone()
            .unwrap_or_else(|| req.working_dir.clone())
    });
    Ok(InvokeResult {
        primary_output: primary,
        outputs: BTreeMap::new(),
    })
}

fn run_meeting_merge(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    // Stub merge: write a minimal Meeting Artifact JSON so Jobs validate end-to-end.
    // Full alignment / speaker matching lands with the real meeting-merge implementation.
    let out = req.output.clone().unwrap_or_else(|| {
        req.output_dir
            .as_ref()
            .map_or_else(
                || req.working_dir.join("meeting.json"),
                |d| d.join("meeting.json"),
            )
    });
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ExecError::Step(format!("meeting-merge mkdir: {e}")))?;
    }

    let participants = req
        .options
        .get("participants")
        .cloned()
        .unwrap_or(ArgValue::Map(BTreeMap::new()));
    let alignment = req
        .options
        .get("alignment")
        .cloned()
        .unwrap_or(ArgValue::Map(BTreeMap::new()));

    let body = serde_json::json!({
        "version": 1,
        "artifact_type": "meeting",
        "stub": true,
        "input": req.input.display().to_string(),
        "alignment": arg_to_json(&alignment),
        "participants": arg_to_json(&participants),
        "notes": "Stub meeting-merge; replace with real alignment when ready.",
    });
    let text = serde_json::to_string_pretty(&body)
        .map_err(|e| ExecError::Step(format!("meeting-merge json: {e}")))?;
    std::fs::write(&out, text)
        .map_err(|e| ExecError::Step(format!("meeting-merge write: {e}")))?;

    Ok(InvokeResult {
        primary_output: out,
        outputs: BTreeMap::new(),
    })
}

fn arg_to_json(v: &ArgValue) -> serde_json::Value {
    match v {
        ArgValue::Bool(b) => serde_json::Value::Bool(*b),
        ArgValue::Number(n) => serde_json::json!(n),
        ArgValue::String(s) => serde_json::Value::String(s.clone()),
        ArgValue::Strings(xs) => {
            serde_json::Value::Array(xs.iter().cloned().map(serde_json::Value::String).collect())
        }
        ArgValue::List(xs) => {
            serde_json::Value::Array(xs.iter().map(arg_to_json).collect())
        }
        ArgValue::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in m {
                obj.insert(k.clone(), arg_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
    }
}

fn run_diarize(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-diarize")?;
    let primary = infer_diarize_output(req);
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
    if req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--overwrite".into());
    }

    run_cmd(&bin, &args, &req.working_dir)?;
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
    if req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--overwrite".into());
    }
    if req.options.get("segments").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--segments".into());
    }
    if req.options.get("word_timestamps").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--word-timestamps".into());
    }
    if let Some(fmt) = req.options.get("format").and_then(ArgValue::as_string) {
        args.push("--format".into());
        args.push(fmt);
    }

    run_cmd(&bin, &args, &req.working_dir)?;
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
    let stem = main
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
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
    run_cmd(&bin, &args, &req.working_dir)?;
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
                "md" | "markdown" | "txt" | "rst" | "pdf" | "docx" | "doc" | "xlsx" | "xls"
                    | "pptx" | "ppt" | "odt" | "ods",
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
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-q".into(),
        "-o".into(),
        primary.display().to_string(),
    ];
    if req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--overwrite".into());
    }
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
    run_cmd(&bin, &args, &req.working_dir)?;
    Ok(InvokeResult {
        primary_output: primary,
        outputs: BTreeMap::new(),
    })
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

fn run_cmd(bin: &Path, args: &[String], cwd: &Path) -> Result<(), ExecError> {
    let output = Command::new(bin)
        .args(args)
        .current_dir(cwd)
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
    if detail.is_empty() {
        Err(ExecError::Step(format!("{} exited {code}", bin.display())))
    } else {
        Err(ExecError::Step(format!(
            "{} exited {code}: {detail}",
            bin.display()
        )))
    }
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
