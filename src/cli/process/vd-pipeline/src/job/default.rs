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

    let mut steps: Vec<WorkflowNode> = vec![Step {
        id: Some("transcript".into()),
        options,
        ..Step::new(Capability::Transcribe)
    }
    .into()];

    if args.docs.is_some() {
        steps.push(Step::new(Capability::PrepareContext).into());
    }

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
            docs: args.docs.clone(),
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

fn overwrite_opts(overwrite: bool) -> BTreeMap<String, ArgValue> {
    let mut o = BTreeMap::new();
    if overwrite {
        o.insert("overwrite".into(), ArgValue::Bool(true));
    }
    o
}
