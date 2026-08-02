//! CLI flags → default Job.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::schema::{
    ArgValue, Capability, Job, JobContext, JobInput, JobOutput, Step, TranscribeEngine,
    WorkflowNode,
};

#[derive(Debug, Clone)]
pub struct DefaultJobArgs {
    pub audio: PathBuf,
    pub engine: TranscribeEngine,
    pub model: Option<String>,
    pub device: Option<String>,
    pub flash: bool,
    /// Optional preprocess `speed` factor (e.g. 2.0–2.2). Remapped via TimeMap.
    pub speed: Option<f64>,
    pub docs: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub continue_on_error: bool,
    pub overwrite: bool,
}

/// True when the path looks like a video container (preprocess should extract audio).
pub fn is_video_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some(
            "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" | "mpeg" | "mpg" | "flv" | "wmv"
        )
    )
}

/// Default ASR preprocess filter chain. Video inputs get `extract-audio` first (ffmpeg).
pub fn default_preprocess_filters(input: &Path, speed: Option<f64>) -> (String, Vec<ArgValue>) {
    let video = is_video_path(input);
    let mut filters = Vec::new();
    if video {
        filters.push(filter_type("extract-audio", &[]));
    }
    filters.push(filter_type(
        "resample",
        &[("rate", ArgValue::Number(16_000.0))],
    ));
    filters.push(filter_type("mono", &[]));
    if let Some(factor) = speed {
        filters.push(filter_type(
            "speed",
            &[("factor", ArgValue::Number(factor))],
        ));
    }
    filters.push(filter_type("trim-silence", &[]));
    filters.push(filter_type("normalize", &[]));
    let provider = if video {
        "ffmpeg".into()
    } else {
        "stub".into()
    };
    (provider, filters)
}

pub fn default_job(args: &DefaultJobArgs) -> Job {
    let (provider, filters) = default_preprocess_filters(&args.audio, args.speed);

    let mut preprocess_opts = BTreeMap::new();
    preprocess_opts.insert("provider".into(), ArgValue::String(provider));
    preprocess_opts.insert("filters".into(), ArgValue::List(filters));
    if args.overwrite {
        preprocess_opts.insert("overwrite".into(), ArgValue::Bool(true));
    }

    let mut options = BTreeMap::new();
    options.insert(
        "engine".into(),
        ArgValue::String(args.engine.as_str().into()),
    );
    if let Some(m) = &args.model {
        options.insert("model".into(), ArgValue::String(m.clone()));
    }
    let device = args.device.clone().or_else(default_transcribe_device);
    if let Some(d) = device {
        options.insert("device".into(), ArgValue::String(d));
    }
    if args.flash {
        options.insert("flash".into(), ArgValue::Bool(true));
    }
    if args.overwrite {
        options.insert("overwrite".into(), ArgValue::Bool(true));
    }

    let mut steps: Vec<WorkflowNode> = vec![
        Step {
            id: Some("prepared".into()),
            options: preprocess_opts,
            ..Step::new(Capability::Preprocess)
        }
        .into(),
        Step {
            id: Some("transcript".into()),
            input: Some("prepared".into()),
            options,
            ..Step::new(Capability::Transcribe)
        }
        .into(),
    ];

    // Always prepare project assets for fix-asr / fix-terms (vd-assets).
    // `--docs` selects the source root; default is `.` (working directory).
    steps.push(Step::new(Capability::PrepareContext).into());

    steps.push(
        Step {
            input: Some("transcript".into()),
            options: overwrite_opts(args.overwrite),
            ..Step::new(Capability::FixCasing)
        }
        .into(),
    );
    steps.push(
        Step {
            options: overwrite_opts(args.overwrite),
            ..Step::new(Capability::FixAsr)
        }
        .into(),
    );
    steps.push(
        Step {
            options: overwrite_opts(args.overwrite),
            ..Step::new(Capability::FixTerms)
        }
        .into(),
    );
    steps.push(
        Step {
            options: overwrite_opts(args.overwrite),
            ..Step::new(Capability::FixLayout)
        }
        .into(),
    );

    Job {
        version: 1,
        name: None,
        working_dir: args.working_dir.clone(),
        input: JobInput {
            audio: Some(args.audio.clone()),
        },
        context: JobContext {
            docs: Some(args.docs.clone().unwrap_or_else(|| PathBuf::from("."))),
            assets: None,
        },
        output: JobOutput {
            dir: args.output_dir.clone(),
        },
        continue_on_error: args.continue_on_error,
        max_parallel: None,
        resources: BTreeMap::new(),
        steps,
    }
}

/// On macOS prefer Metal for ASR when the caller did not set `device`.
fn default_transcribe_device() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        Some("metal".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn filter_type(op: &str, extra: &[(&str, ArgValue)]) -> ArgValue {
    let mut m = BTreeMap::new();
    m.insert("type".into(), ArgValue::String(op.into()));
    for (k, v) in extra {
        m.insert((*k).into(), v.clone());
    }
    ArgValue::Map(m)
}

fn overwrite_opts(overwrite: bool) -> BTreeMap<String, ArgValue> {
    let mut o = BTreeMap::new();
    if overwrite {
        o.insert("overwrite".into(), ArgValue::Bool(true));
    }
    o
}
