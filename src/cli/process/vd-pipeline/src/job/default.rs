//! CLI flags → default Job.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::schema::{
    ArgValue, Capability, Job, JobContext, JobInput, JobOutput, Step, TranscribeEngine,
};

#[derive(Debug, Clone)]
pub struct DefaultJobArgs {
    pub audio: PathBuf,
    pub engine: TranscribeEngine,
    pub model: Option<String>,
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
    if args.overwrite {
        options.insert("overwrite".into(), ArgValue::Bool(true));
    }

    let mut steps = vec![Step {
        r#use: Capability::Transcribe,
        id: Some("transcript".into()),
        name: None,
        input: None,
        output: None,
        skip: false,
        options,
    }];

    if args.docs.is_some() {
        steps.push(Step {
            r#use: Capability::PrepareContext,
            id: None,
            name: None,
            input: None,
            output: None,
            skip: false,
            options: BTreeMap::new(),
        });
    }

    steps.push(Step {
        r#use: Capability::FixCasing,
        id: None,
        name: None,
        input: Some("transcript".into()),
        output: None,
        skip: false,
        options: overwrite_opts(args.overwrite),
    });
    steps.push(Step {
        r#use: Capability::FixAsr,
        id: None,
        name: None,
        input: None,
        output: None,
        skip: false,
        options: overwrite_opts(args.overwrite),
    });
    steps.push(Step {
        r#use: Capability::FixTerms,
        id: None,
        name: None,
        input: None,
        output: None,
        skip: false,
        options: overwrite_opts(args.overwrite),
    });

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
