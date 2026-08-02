//! Binder that emits Node*/Artifact Runtime events while a Job runs.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use vd_pipeline::{Binder, ExecError, InvokeRequest, InvokeResult, SubprocessBinder};

use crate::engine::Engine;
use crate::store::{now_rfc3339, EventRecord, NodeStatus};

pub struct ObservingBinder {
    pub engine: Engine,
    pub job_id: String,
    pub inner: SubprocessBinder,
    /// Parallel branches may interleave; pair by capability + Ready status.
    pub seq: AtomicUsize,
}

impl ObservingBinder {
    pub fn new(engine: Engine, job_id: String) -> Self {
        Self {
            engine,
            job_id,
            inner: SubprocessBinder,
            seq: AtomicUsize::new(0),
        }
    }
}

impl Binder for ObservingBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        let _n = self.seq.fetch_add(1, Ordering::SeqCst);
        let cap = req.capability.as_str();
        let node_id = self
            .engine
            .mark_node_started(&self.job_id, cap)
            .unwrap_or_else(|| format!("{cap}-{}", _n));

        let mut fields = BTreeMap::new();
        fields.insert(
            "capability".into(),
            serde_json::Value::String(cap.to_string()),
        );
        fields.insert(
            "input".into(),
            serde_json::Value::String(req.input.display().to_string()),
        );
        let _ = self.engine.append_event(
            &self.job_id,
            EventRecord {
                ts: now_rfc3339(),
                kind: "NodeStarted".into(),
                node_id: Some(node_id.clone()),
                message: None,
                fields: fields.clone(),
            },
        );

        match self.inner.invoke(req) {
            Ok(result) => {
                let mut art_fields = BTreeMap::new();
                art_fields.insert(
                    "path".into(),
                    serde_json::Value::String(result.primary_output.display().to_string()),
                );
                art_fields.insert(
                    "capability".into(),
                    serde_json::Value::String(cap.to_string()),
                );
                let _ = self.engine.append_event(
                    &self.job_id,
                    EventRecord {
                        ts: now_rfc3339(),
                        kind: "ArtifactProduced".into(),
                        node_id: Some(node_id.clone()),
                        message: Some(result.primary_output.display().to_string()),
                        fields: art_fields,
                    },
                );
                let _ = self.engine.mark_node_finished(
                    &self.job_id,
                    &node_id,
                    NodeStatus::Completed,
                    None,
                );
                let _ = self.engine.append_event(
                    &self.job_id,
                    EventRecord {
                        ts: now_rfc3339(),
                        kind: "NodeCompleted".into(),
                        node_id: Some(node_id),
                        message: None,
                        fields,
                    },
                );
                Ok(result)
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = self.engine.mark_node_finished(
                    &self.job_id,
                    &node_id,
                    NodeStatus::Failed,
                    Some(msg.clone()),
                );
                let _ = self.engine.append_event(
                    &self.job_id,
                    EventRecord {
                        ts: now_rfc3339(),
                        kind: "NodeFailed".into(),
                        node_id: Some(node_id),
                        message: Some(msg),
                        fields,
                    },
                );
                Err(e)
            }
        }
    }
}

impl Binder for &ObservingBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        (*self).invoke(req)
    }
}