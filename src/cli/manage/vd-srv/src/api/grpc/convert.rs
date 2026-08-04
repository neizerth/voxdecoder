//! Map Engine / store types ↔ gRPC protobuf (observe path).

use crate::store::{
    EventRecord, JobRecord, JobStatus as StoreJobStatus, NodeRecord, NodeStatus as StoreNodeStatus,
    Priority as StorePriority,
};

use super::pb::{
    self, ArtifactProduced, Event, HealthResponse, JobCancelled, JobCompleted, JobFailed,
    JobFinished, JobQueued, JobStarted, JobStatus, JobView, JobWaitingResources, ListJobsResponse,
    NodeCompleted, NodeFailed, NodeProgress, NodeStarted, NodeStatus, NodeView, Priority,
    UnknownEvent,
};

pub fn health_response(
    workers: u32,
    workers_busy: u32,
    data_dir: impl Into<String>,
    resources: &serde_json::Value,
) -> HealthResponse {
    HealthResponse {
        workers,
        workers_busy,
        data_dir: data_dir.into(),
        resources_json: serde_json::to_string(resources).unwrap_or_else(|_| "{}".into()),
    }
}

pub fn job_record_to_view(rec: &JobRecord) -> JobView {
    JobView {
        id: rec.id.clone(),
        status: job_status_to_pb(rec.status).into(),
        priority: priority_to_pb(rec.priority).into(),
        created_at: Some(rec.created_at.clone()),
        queued_at: rec.queued_at.clone(),
        started_at: rec.started_at.clone(),
        finished_at: rec.finished_at.clone(),
        exit_code: rec.exit_code,
        error: rec.error.clone(),
        progress: rec.progress.map(u32::from),
        phase: rec.phase.clone(),
        processed: rec.processed,
        total: rec.total,
        unit: rec.unit.clone(),
        nodes: rec.nodes.iter().map(node_record_to_view).collect(),
    }
}

pub fn list_jobs_response(recs: &[JobRecord]) -> ListJobsResponse {
    ListJobsResponse {
        jobs: recs.iter().map(job_record_to_view).collect(),
    }
}

pub fn event_record_to_pb(ev: &EventRecord) -> Event {
    let mut out = Event {
        ts: ev.ts.clone(),
        node_id: ev.node_id.clone(),
        message: ev.message.clone(),
        payload: None,
    };
    out.payload = Some(match ev.kind.as_str() {
        "JobQueued" => pb::event::Payload::JobQueued(JobQueued {}),
        "JobStarted" => pb::event::Payload::JobStarted(JobStarted {}),
        "JobWaitingResources" => pb::event::Payload::JobWaitingResources(JobWaitingResources {}),
        "JobFinished" => pb::event::Payload::JobFinished(JobFinished {}),
        "JobCompleted" => pb::event::Payload::JobCompleted(JobCompleted {}),
        "JobFailed" => pb::event::Payload::JobFailed(JobFailed {}),
        "JobCancelled" => pb::event::Payload::JobCancelled(JobCancelled {}),
        "NodeStarted" => pb::event::Payload::NodeStarted(NodeStarted {}),
        "NodeCompleted" => pb::event::Payload::NodeCompleted(NodeCompleted {}),
        "NodeFailed" => pb::event::Payload::NodeFailed(NodeFailed {}),
        "NodeProgress" => pb::event::Payload::NodeProgress(node_progress_from_fields(ev)),
        "ArtifactProduced" => pb::event::Payload::ArtifactProduced(artifact_from_fields(ev)),
        other => pb::event::Payload::Unknown(UnknownEvent {
            kind: other.to_string(),
            fields_json: fields_json(ev),
        }),
    });
    out
}

fn node_record_to_view(n: &NodeRecord) -> NodeView {
    NodeView {
        id: n.id.clone(),
        leaf_index: n.leaf_index as u32,
        capability: n.capability.clone(),
        status: node_status_to_pb(n.status).into(),
        depends_on: n.depends_on.clone(),
        started_at: n.started_at.clone(),
        finished_at: n.finished_at.clone(),
        error: n.error.clone(),
    }
}

fn job_status_to_pb(s: StoreJobStatus) -> JobStatus {
    match s {
        StoreJobStatus::Submitted => JobStatus::Submitted,
        StoreJobStatus::Queued => JobStatus::Queued,
        StoreJobStatus::WaitingResources => JobStatus::WaitingResources,
        StoreJobStatus::Scheduled => JobStatus::Scheduled,
        StoreJobStatus::Running => JobStatus::Running,
        StoreJobStatus::Completed => JobStatus::Completed,
        StoreJobStatus::Failed => JobStatus::Failed,
        StoreJobStatus::Cancelled => JobStatus::Cancelled,
    }
}

fn node_status_to_pb(s: StoreNodeStatus) -> NodeStatus {
    match s {
        StoreNodeStatus::Pending => NodeStatus::Pending,
        StoreNodeStatus::WaitingDependencies => NodeStatus::WaitingDependencies,
        StoreNodeStatus::WaitingResources => NodeStatus::WaitingResources,
        StoreNodeStatus::Ready => NodeStatus::Ready,
        StoreNodeStatus::Running => NodeStatus::Running,
        StoreNodeStatus::Completed => NodeStatus::Completed,
        StoreNodeStatus::Failed => NodeStatus::Failed,
        StoreNodeStatus::Cancelled => NodeStatus::Cancelled,
        StoreNodeStatus::Skipped => NodeStatus::Skipped,
    }
}

fn priority_to_pb(p: StorePriority) -> Priority {
    match p {
        StorePriority::Low => Priority::Low,
        StorePriority::Normal => Priority::Normal,
        StorePriority::High => Priority::High,
    }
}

fn fields_json(ev: &EventRecord) -> String {
    if ev.fields.is_empty() {
        return "{}".into();
    }
    serde_json::to_string(&ev.fields).unwrap_or_else(|_| "{}".into())
}

fn field_str(ev: &EventRecord, key: &str) -> Option<String> {
    ev.fields
        .get(key)
        .and_then(|v| v.as_str().map(str::to_string))
}

fn field_u64(ev: &EventRecord, key: &str) -> Option<u64> {
    ev.fields.get(key).and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_i64().map(|i| i as u64))
            .or_else(|| v.as_f64().map(|f| f as u64))
    })
}

fn node_progress_from_fields(ev: &EventRecord) -> NodeProgress {
    NodeProgress {
        percent: field_u64(ev, "percent")
            .or_else(|| field_u64(ev, "progress"))
            .map(|p| p.min(100) as u32),
        phase: field_str(ev, "phase"),
        processed: field_u64(ev, "processed"),
        total: field_u64(ev, "total"),
        unit: field_str(ev, "unit"),
    }
}

fn artifact_from_fields(ev: &EventRecord) -> ArtifactProduced {
    ArtifactProduced {
        artifact_id: field_str(ev, "id").or_else(|| field_str(ev, "artifact_id")),
        path: field_str(ev, "path"),
        kind: field_str(ev, "kind"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::now_rfc3339;
    use std::collections::BTreeMap;

    #[test]
    fn maps_known_and_unknown_events() {
        let known = EventRecord {
            ts: now_rfc3339(),
            kind: "JobStarted".into(),
            node_id: None,
            message: Some("go".into()),
            fields: BTreeMap::new(),
        };
        let pb = event_record_to_pb(&known);
        assert!(matches!(
            pb.payload,
            Some(pb::event::Payload::JobStarted(_))
        ));

        let unknown = EventRecord {
            ts: now_rfc3339(),
            kind: "CustomThing".into(),
            node_id: Some("n1".into()),
            message: None,
            fields: BTreeMap::from([("x".into(), serde_json::json!(1))]),
        };
        let pb = event_record_to_pb(&unknown);
        match pb.payload {
            Some(pb::event::Payload::Unknown(u)) => {
                assert_eq!(u.kind, "CustomThing");
                assert!(u.fields_json.contains('1'));
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }
}
