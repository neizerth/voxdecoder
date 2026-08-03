//! postprocess capability resolves / options.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vd_pipeline::{resolve_job, ArgValue, Capability, Job, JobInput, Step};

#[test]
fn postprocess_requires_recipes() {
    let job = Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput::default(),
        context: Default::default(),
        output: Default::default(),
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![Step::new(Capability::Postprocess)]
            .into_iter()
            .map(Into::into)
            .collect(),
    };
    let err = resolve_job(job).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("recipes"));
}

#[test]
fn postprocess_with_recipes_ok() {
    let mut options = BTreeMap::new();
    options.insert(
        "recipes".into(),
        ArgValue::Strings(vec!["./summary.yaml".into()]),
    );
    let mut inputs = BTreeMap::new();
    inputs.insert("transcript".into(), ArgValue::String("t.txt".into()));
    options.insert("inputs".into(), ArgValue::Map(inputs));
    options.insert(
        "provider".into(),
        ArgValue::Map(BTreeMap::from([(
            "type".into(),
            ArgValue::String("stub".into()),
        )])),
    );

    let mut step = Step::new(Capability::Postprocess);
    step.id = Some("summary".into());
    step.options = options;

    let job = Job {
        version: 1,
        name: None,
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput::default(),
        context: Default::default(),
        output: Default::default(),
        max_parallel: None,
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![step].into_iter().map(Into::into).collect(),
    };
    resolve_job(job).expect("postprocess with recipes should resolve");
}
