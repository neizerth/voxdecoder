//! Load Job from yaml/json file.

use std::fs;
use std::path::Path;

use super::schema::{Job, JobError};

pub fn load_job_file(path: &Path) -> Result<Job, JobError> {
    let text = fs::read_to_string(path)
        .map_err(|e| JobError::NotFound(format!("{}: {e}", path.display())))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let job: Job = match ext.as_str() {
        "json" => serde_json::from_str(&text)
            .map_err(|e| JobError::Usage(format!("invalid job json: {e}")))?,
        "yaml" | "yml" => serde_yaml::from_str(&text)
            .map_err(|e| JobError::Usage(format!("invalid job yaml: {e}")))?,
        _ => {
            return Err(JobError::Usage(format!(
                "unsupported job file extension: {ext} (use .yaml / .yml / .json)"
            )));
        }
    };
    if job.version != 1 {
        return Err(JobError::Usage(format!(
            "unsupported job version: {} (expected 1)",
            job.version
        )));
    }
    if job.leaf_count() == 0 {
        return Err(JobError::Usage("job has no steps".into()));
    }
    Ok(job)
}
