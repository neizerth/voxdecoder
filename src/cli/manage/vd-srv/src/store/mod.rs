//! Filesystem Job Store / Event Store / artifacts.

mod types;

pub use types::*;

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vd_pipeline::{resolve_job, Capability, Job, ResolvedJob};

use crate::paths;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    NotFound(String),
}

impl StoreError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::NotFound(_) => 3,
            Self::Io(_) => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobStore {
    pub root: PathBuf,
}

impl JobStore {
    pub fn open(data_dir: &Path) -> Result<Self, StoreError> {
        let root = paths::jobs_dir(data_dir);
        fs::create_dir_all(&root).map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(Self { root })
    }

    pub fn job_dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    pub fn create(
        &self,
        job: Job,
        priority: Priority,
        restart: RestartPolicy,
    ) -> Result<JobRecord, StoreError> {
        let resolved = resolve_job(job.clone()).map_err(|e| StoreError::Usage(e.to_string()))?;
        let id = new_job_id();
        let dir = self.job_dir(&id);
        fs::create_dir_all(&dir).map_err(|e| StoreError::Io(e.to_string()))?;

        let now = now_rfc3339();
        let nodes = nodes_from_resolved(&resolved);
        let record = JobRecord {
            id: id.clone(),
            status: JobStatus::Queued,
            priority,
            restart,
            created_at: now.clone(),
            queued_at: Some(now),
            started_at: None,
            finished_at: None,
            exit_code: None,
            error: None,
            job,
            nodes,
            working_dir: resolved.working_dir.clone(),
        };

        self.write_job_yaml(&dir, &record.job)?;
        self.write_state(&record)?;
        self.write_json(&dir.join("resolved.json"), &resolved_summary(&resolved))?;
        self.write_json(&dir.join("artifacts.json"), &Vec::<ArtifactEntry>::new())?;
        File::create(dir.join("events.ndjson")).map_err(|e| StoreError::Io(e.to_string()))?;
        File::create(dir.join("stdout.log")).map_err(|e| StoreError::Io(e.to_string()))?;
        File::create(dir.join("stderr.log")).map_err(|e| StoreError::Io(e.to_string()))?;

        self.append_event(
            &id,
            EventRecord {
                ts: now_rfc3339(),
                kind: "JobQueued".into(),
                node_id: None,
                message: None,
                fields: Default::default(),
            },
        )?;

        Ok(record)
    }

    pub fn load(&self, id: &str) -> Result<JobRecord, StoreError> {
        let path = self.job_dir(id).join("state.json");
        if !path.exists() {
            return Err(StoreError::NotFound(format!("job not found: {id}")));
        }
        let body = fs::read_to_string(&path).map_err(|e| StoreError::Io(e.to_string()))?;
        serde_json::from_str(&body).map_err(|e| StoreError::Io(e.to_string()))
    }

    pub fn save(&self, record: &JobRecord) -> Result<(), StoreError> {
        self.write_state(record)
    }

    pub fn list_ids(&self) -> Result<Vec<String>, StoreError> {
        let mut ids = Vec::new();
        if !self.root.exists() {
            return Ok(ids);
        }
        for ent in fs::read_dir(&self.root).map_err(|e| StoreError::Io(e.to_string()))? {
            let ent = ent.map_err(|e| StoreError::Io(e.to_string()))?;
            if ent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                ids.push(ent.file_name().to_string_lossy().into_owned());
            }
        }
        ids.sort();
        Ok(ids)
    }

    pub fn list(&self) -> Result<Vec<JobRecord>, StoreError> {
        let mut out = Vec::new();
        for id in self.list_ids()? {
            if let Ok(r) = self.load(&id) {
                out.push(r);
            }
        }
        Ok(out)
    }

    pub fn append_event(&self, id: &str, event: EventRecord) -> Result<(), StoreError> {
        let path = self.job_dir(id).join("events.ndjson");
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| StoreError::Io(e.to_string()))?;
        let line = serde_json::to_string(&event).map_err(|e| StoreError::Io(e.to_string()))?;
        writeln!(f, "{line}").map_err(|e| StoreError::Io(e.to_string()))
    }

    pub fn read_events(&self, id: &str) -> Result<Vec<EventRecord>, StoreError> {
        let path = self.job_dir(id).join("events.ndjson");
        if !path.exists() {
            return Err(StoreError::NotFound(format!("job not found: {id}")));
        }
        let f = File::open(&path).map_err(|e| StoreError::Io(e.to_string()))?;
        let mut out = Vec::new();
        for line in BufReader::new(f).lines() {
            let line = line.map_err(|e| StoreError::Io(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            out.push(serde_json::from_str(&line).map_err(|e| StoreError::Io(e.to_string()))?);
        }
        Ok(out)
    }

    pub fn read_artifacts(&self, id: &str) -> Result<Vec<ArtifactEntry>, StoreError> {
        let path = self.job_dir(id).join("artifacts.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let body = fs::read_to_string(&path).map_err(|e| StoreError::Io(e.to_string()))?;
        serde_json::from_str(&body).map_err(|e| StoreError::Io(e.to_string()))
    }

    pub fn write_artifacts(&self, id: &str, arts: &[ArtifactEntry]) -> Result<(), StoreError> {
        self.write_json(&self.job_dir(id).join("artifacts.json"), &arts.to_vec())
    }

    /// Resolve `artifact_id` or `job_id:artifact_id` / `job_id/artifact_id` to a path.
    pub fn resolve_artifact(&self, artifact_ref: &str) -> Result<PathBuf, StoreError> {
        let scoped = artifact_ref
            .split_once(':')
            .or_else(|| artifact_ref.split_once('/'));
        if let Some((job_id, art_id)) = scoped {
            return self
                .read_artifacts(job_id)?
                .into_iter()
                .find(|a| a.id == art_id)
                .map(|a| a.path)
                .ok_or_else(|| {
                    StoreError::NotFound(format!(
                        "artifact not found: {art_id} in job {job_id}"
                    ))
                });
        }

        let mut matches = Vec::new();
        for job_id in self.list_ids()? {
            for entry in self.read_artifacts(&job_id)? {
                if entry.id == artifact_ref {
                    matches.push((job_id.clone(), entry.path));
                }
            }
        }
        match matches.as_slice() {
            [(_, path)] => Ok(path.clone()),
            [] => Err(StoreError::NotFound(format!(
                "artifact not found: {artifact_ref}"
            ))),
            _ => Err(StoreError::Usage(format!(
                "ambiguous artifact id `{artifact_ref}`; use job_id:artifact_id"
            ))),
        }
    }

    pub fn read_log(&self, id: &str, stderr: bool) -> Result<String, StoreError> {
        let name = if stderr { "stderr.log" } else { "stdout.log" };
        let path = self.job_dir(id).join(name);
        fs::read_to_string(&path).map_err(|e| StoreError::Io(e.to_string()))
    }

    fn write_state(&self, record: &JobRecord) -> Result<(), StoreError> {
        self.write_json(&self.job_dir(&record.id).join("state.json"), record)
    }

    fn write_job_yaml(&self, dir: &Path, job: &Job) -> Result<(), StoreError> {
        let body = serde_yaml::to_string(job).map_err(|e| StoreError::Io(e.to_string()))?;
        fs::write(dir.join("job.yaml"), body).map_err(|e| StoreError::Io(e.to_string()))
    }

    fn write_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), StoreError> {
        let body =
            serde_json::to_string_pretty(value).map_err(|e| StoreError::Io(e.to_string()))?;
        fs::write(path, body).map_err(|e| StoreError::Io(e.to_string()))
    }
}

use serde::Serialize;

fn nodes_from_resolved(resolved: &ResolvedJob) -> Vec<NodeRecord> {
    let n = resolved.steps.len();
    // Sequential dependency hint from legacy order: each leaf waits on previous in topo order.
    let order = &resolved.order;
    let mut pred: Vec<Option<usize>> = vec![None; n];
    for w in order.windows(2) {
        pred[w[1]] = Some(w[0]);
    }

    resolved
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let id = step
                .id
                .clone()
                .unwrap_or_else(|| format!("leaf-{}", step.index));
            let depends_on: Vec<String> = pred[i]
                .map(|p| {
                    resolved.steps[p]
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("leaf-{}", resolved.steps[p].index))
                })
                .into_iter()
                .collect();
            let status = if step.skip {
                NodeStatus::Skipped
            } else if depends_on.is_empty() {
                NodeStatus::Ready
            } else {
                NodeStatus::WaitingDependencies
            };
            NodeRecord {
                id,
                leaf_index: i,
                capability: capability_str(step.capability).into(),
                status,
                depends_on,
                resources: default_resources(step.capability),
                started_at: None,
                finished_at: None,
                error: None,
            }
        })
        .collect()
}

fn capability_str(c: Capability) -> &'static str {
    c.as_str()
}

fn default_resources(c: Capability) -> std::collections::BTreeMap<String, u32> {
    let mut m = std::collections::BTreeMap::new();
    match c {
        Capability::Transcribe | Capability::Diarize => {
            m.insert("cpu".into(), 1);
        }
        _ => {
            m.insert("cpu".into(), 1);
        }
    }
    m
}

fn resolved_summary(resolved: &ResolvedJob) -> serde_json::Value {
    serde_json::json!({
        "working_dir": resolved.working_dir,
        "leaf_count": resolved.steps.len(),
        "order": resolved.order,
        "steps": resolved.steps.iter().map(|s| {
            serde_json::json!({
                "index": s.index,
                "path": s.path,
                "id": s.id,
                "capability": capability_str(s.capability),
            })
        }).collect::<Vec<_>>(),
    })
}

fn new_job_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let rumble = std::process::id();
    format!("job-{nanos:x}-{rumble:x}")
}

pub fn now_rfc3339() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Compact UTC-ish stamp for persistence (not full RFC3339 tz).
    format!("{}.{:03}Z", dur.as_secs(), dur.subsec_millis())
}
