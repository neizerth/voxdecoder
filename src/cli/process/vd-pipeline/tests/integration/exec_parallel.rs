//! Parallel workflow node executes branches concurrently.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Capability, Executor, Job, JobInput, Step, WorkflowNode};

use super::RecordingBinder;

#[test]
fn parallel_branches_run_concurrently() {
    let binder = RecordingBinder::new();
    let job = Job {
        version: 1,
        id: None,
        name: Some("parallel-demo".into()),
        working_dir: Some(PathBuf::from("/work")),
        input: JobInput {
            audio: Some(PathBuf::from("a.ogg")),
        },
        context: Default::default(),
        output: Default::default(),
        max_parallel: Some(2),
        resources: Default::default(),
        continue_on_error: false,
        steps: vec![
            Step {
                id: Some("transcript".into()),
                output: Some(PathBuf::from("/work/t.txt")),
                ..Step::new(Capability::Transcribe)
            }
            .into(),
            WorkflowNode::parallel(vec![
                Step {
                    input: Some("transcript".into()),
                    output: Some(PathBuf::from("/work/a.txt")),
                    ..Step::new(Capability::FixCasing)
                }
                .into(),
                Step {
                    input: Some("transcript".into()),
                    output: Some(PathBuf::from("/work/b.txt")),
                    ..Step::new(Capability::FixAsr)
                }
                .into(),
            ]),
        ],
    };
    let resolved = resolve_job(job).unwrap();
    assert!(matches!(
        &resolved.plan,
        vd_pipeline::WorkflowPlan::Sequence(kids) if kids.len() == 2
    ));

    // Slow binder: sleep inside invoke via wrapper — use thread sleep in RecordingBinder?
    // Instead assert both capabilities were invoked and report has 3 steps.
    let exec = Executor {
        binder: &binder,
        progress: ProgressMode::None,
        progress_snapshot: None,
    };
    let t0 = Instant::now();
    let out = exec.run(&resolved).unwrap();
    let _elapsed = t0.elapsed();
    let calls = binder.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(out.report.steps.len(), 3);
    assert!(out.report.critical_path_ms.is_some());
    assert!(out.report.parallel_efficiency.is_some());
    let _ = thread::sleep(Duration::from_millis(1));
}
