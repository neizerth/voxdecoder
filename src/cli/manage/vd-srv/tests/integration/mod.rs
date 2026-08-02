//! Engine integration (in-process, no socket).

#![allow(clippy::default_trait_access, clippy::field_reassign_with_default)]

use std::thread;
use std::time::Duration;

use tempfile::TempDir;
use vd_pipeline::{Capability, Job, JobInput, Step};
use vd_srv::config::ServerConfig;
use vd_srv::store::{JobStatus, Priority, RestartPolicy};
use vd_srv::Engine;

fn sample_job(dir: &std::path::Path) -> Job {
    let sample = dir.join("sample.txt");
    std::fs::write(&sample, "hello world\n").unwrap();
    Job {
        version: 1,
        name: Some("fix".into()),
        working_dir: Some(dir.to_path_buf()),
        input: JobInput::default(),
        context: Default::default(),
        output: Default::default(),
        continue_on_error: false,
        max_parallel: Some(1),
        resources: Default::default(),
        steps: vec![
            {
                let mut s = Step::new(Capability::FixCasing);
                s.input = Some(sample.display().to_string());
                s.options.insert(
                    "overwrite".into(),
                    vd_pipeline::ArgValue::Bool(true),
                );
                s.into()
            },
        ],
    }
}

#[test]
fn submit_runs_to_completion_or_fail() {
    let dir = TempDir::new().unwrap();
    let mut cfg = ServerConfig::default();
    cfg.workers = 1;
    let engine = Engine::start(dir.path().to_path_buf(), cfg).unwrap();
    let rec = engine
        .submit(
            sample_job(dir.path()),
            Priority::Normal,
            RestartPolicy::Resume,
        )
        .unwrap();

    let mut last = JobStatus::Queued;
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(100));
        let j = engine.job(&rec.id).unwrap();
        last = j.status;
        if j.status.is_terminal() {
            break;
        }
    }
    // May complete (if vd-fix-casing on PATH) or fail — both prove engine advanced.
    assert!(
        matches!(
            last,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        ),
        "unexpected status {last:?}"
    );
    engine.stop();
}
