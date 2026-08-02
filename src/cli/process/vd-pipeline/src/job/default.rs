//! CLI flags → default Job.

use std::collections::BTreeMap;
use std::path::PathBuf;

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
    pub docs: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub continue_on_error: bool,
    pub overwrite: bool,
}

pub fn default_job(args: &DefaultJobArgs) -> Job {
    let mut preprocess_opts = BTreeMap::new();
    preprocess_opts.insert("provider".into(), ArgValue::String("stub".into()));
    preprocess_opts.insert(
        "filters".into(),
        ArgValue::List(vec![
            filter_type("resample", &[("rate", ArgValue::Number(16_000.0))]),
            filter_type("mono", &[]),
            filter_type("trim-silence", &[]),
            filter_type("normalize", &[]),
        ]),
    );
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
    if let Some(d) = &args.device {
        options.insert("device".into(), ArgValue::String(d.clone()));
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
