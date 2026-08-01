//! options forwarded to binder.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, ArgValue, Capability, Executor, Job, JobInput, Step};

use super::RecordingBinder;

#[test]
fn options_forwarded_untouched() {
    let binder = RecordingBinder::new();
    let mut options = BTreeMap::new();
    options.insert("engine".into(), ArgValue::String("gigaam".into()));
    options.insert("model".into(), ArgValue::String("v2_rnnt".into()));
    options.insert("flash".into(), ArgValue::Bool(true));

    let job = Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("a.ogg")),
        },
        context: Default::default(),
        output: Default::default(),
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![Step {
            output: Some(PathBuf::from("/work/t.txt")),
            options: options.clone(),
            ..Step::new(Capability::Transcribe)
        }],
    };
    let resolved = resolve_job(job).unwrap();
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
    };
    exec.run(&resolved).unwrap();
    assert_eq!(binder.calls.borrow()[0].options, options);
}
