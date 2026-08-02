//! Submit planned Job to the shared Executor.

use vd_pipeline::progress::ProgressMode;
use vd_pipeline::{resolve_job, Executor, Job, SubprocessBinder};

use super::PlanError;

pub fn submit_job(job: Job, progress: ProgressMode) -> Result<std::path::PathBuf, PlanError> {
    let resolved = resolve_job(job).map_err(|e| {
        let msg = e.to_string();
        if e.exit_code() == 3 {
            PlanError::NotFound(msg)
        } else if e.exit_code() == 2 {
            PlanError::Usage(msg)
        } else {
            PlanError::Other(msg)
        }
    })?;
    let exec = Executor {
        binder: SubprocessBinder,
        progress,
        progress_snapshot: None,
    };
    exec.run(&resolved)
        .map(|o| o.output)
        .map_err(|e| PlanError::Other(e.to_string()))
}
