//! JSON-RPC method handlers for `vd-srv`.

use serde_json::{json, Value};
use vd_pipeline::Job;

use crate::engine::Engine;
use crate::plan::{plan_audio, plan_meeting, wants_execute, AudioRequest, MeetingPlanRequest};
use crate::store::{Priority, RestartPolicy};

use super::rpc::{code, ErrorObject, Request, Response};

pub fn handle(engine: &Engine, req: Request) -> Option<Response> {
    let id = req.id.clone();
    if req.jsonrpc != "2.0" {
        return Some(Response::failure(
            id,
            ErrorObject::invalid_request("jsonrpc must be \"2.0\""),
        ));
    }
    if req.is_notification() {
        // Server ignores client notifications for now.
        return None;
    }

    let result = match req.method.as_str() {
        "server.ping" => Ok(json!({"pong": true})),
        "server.version" => Ok(json!({
            "name": "vd-srv",
            "version": env!("CARGO_PKG_VERSION"),
        })),
        "server.health" => Ok(health(engine)),
        "server.info" => Ok(server_info(engine)),
        "server.stop" => {
            engine.stop();
            Ok(json!({"stopping": true}))
        }
        "job.submit" => job_submit(engine, req.params.as_ref()),
        "plan.audio" => plan_audio_request(engine, req.params.as_ref()),
        "plan.meeting" => plan_meeting_request(engine, req.params.as_ref()),
        "job.cancel" => job_id_op(engine, req.params.as_ref(), |e, id| {
            e.cancel(id)
                .map(|j| serde_json::to_value(j).unwrap_or_default())
        }),
        "job.status" => job_id_op(engine, req.params.as_ref(), |e, id| {
            e.job(id)
                .map(|j| serde_json::to_value(j).unwrap_or_default())
        }),
        "job.list" => engine
            .list()
            .map(|jobs| serde_json::to_value(jobs).unwrap_or_default())
            .map_err(|e| ErrorObject::application(e.to_string())),
        "job.events" => job_id_op(engine, req.params.as_ref(), |e, id| {
            e.events(id)
                .map(|ev| serde_json::to_value(ev).unwrap_or_default())
        }),
        "job.logs" => job_logs(engine, req.params.as_ref()),
        "queue.status" => engine
            .list()
            .map(|jobs| queue_view(&jobs))
            .map_err(|e| ErrorObject::application(e.to_string())),
        "worker.list" => Ok(json!({
            "busy": engine.workers_busy(),
            "workers": engine.workers_total(),
        })),
        "artifact.list" => job_id_op(engine, req.params.as_ref(), |e, id| {
            e.artifacts(id)
                .map(|a| serde_json::to_value(a).unwrap_or_default())
        }),
        "artifact.info" => job_id_op(engine, req.params.as_ref(), |e, id| {
            // Single-job artifact list stands in for info until Artifact Store indexes deepen.
            e.artifacts(id)
                .map(|a| serde_json::to_value(a).unwrap_or_default())
        }),
        "subscribe" => subscribe(req.params.as_ref()),
        "unsubscribe" => Ok(json!({"unsubscribed": true})),
        // Pause / resume reserved; Job Store does not yet gate Running nodes.
        "job.pause" | "job.resume" => Err(ErrorObject::new(
            code::APPLICATION,
            format!("{} is not implemented yet", req.method),
        )),
        other => Err(ErrorObject::method_not_found(other)),
    };

    Some(match result {
        Ok(v) => Response::success(id, v),
        Err(e) => Response::failure(id, e),
    })
}

fn health(engine: &Engine) -> Value {
    json!({
        "workers_busy": engine.workers_busy(),
        "workers": engine.workers_total(),
        "resources": engine.resources_snapshot(),
        "data_dir": engine.data_dir(),
    })
}

fn server_info(engine: &Engine) -> Value {
    json!({
        "name": "vd-srv",
        "version": env!("CARGO_PKG_VERSION"),
        "api_version": "0.1",
        "planners": ["audio", "meeting"],
        "capabilities": ["preprocess", "transcribe", "prepare_context", "fix_casing", "fix_asr", "fix_terms", "diarize", "meeting"],
        "models": [],
        "runners": [],
        "resource_classes": [],
        "health": health(engine),
    })
}

fn plan_audio_request(engine: &Engine, params: Option<&Value>) -> Result<Value, ErrorObject> {
    let params = params.ok_or_else(|| ErrorObject::invalid_params("params required"))?;
    let request: AudioRequest = serde_json::from_value(params.clone())
        .map_err(|e| ErrorObject::invalid_params(format!("invalid audio request: {e}")))?;
    let store = engine
        .job_store()
        .map_err(|e| ErrorObject::application(e.to_string()))?;
    let job = plan_audio(&request, &engine.data_dir(), Some(&store))
        .map_err(|e| ErrorObject::application(e.to_string()))?;
    if !wants_execute(params) {
        return Ok(json!({ "job": job }));
    }
    engine
        .submit(job, Priority::default(), RestartPolicy::default())
        .map(|record| serde_json::to_value(record).unwrap_or_default())
        .map_err(|e| ErrorObject::application(e.to_string()))
}

fn plan_meeting_request(engine: &Engine, params: Option<&Value>) -> Result<Value, ErrorObject> {
    let params = params.ok_or_else(|| ErrorObject::invalid_params("params required"))?;
    let request: MeetingPlanRequest = serde_json::from_value(params.clone())
        .map_err(|e| ErrorObject::invalid_params(format!("invalid meeting request: {e}")))?;
    let store = engine
        .job_store()
        .map_err(|e| ErrorObject::application(e.to_string()))?;
    let job = plan_meeting(&request, &engine.data_dir(), Some(&store))
        .map_err(|e| ErrorObject::application(e.to_string()))?;
    if !wants_execute(params) {
        return Ok(json!({ "job": job }));
    }
    engine
        .submit(job, Priority::default(), RestartPolicy::default())
        .map(|record| serde_json::to_value(record).unwrap_or_default())
        .map_err(|e| ErrorObject::application(e.to_string()))
}

fn job_submit(engine: &Engine, params: Option<&Value>) -> Result<Value, ErrorObject> {
    let p = params.ok_or_else(|| ErrorObject::invalid_params("params required"))?;
    let job = if let Some(j) = p.get("job") {
        serde_json::from_value(j.clone())
            .map_err(|e| ErrorObject::invalid_params(format!("invalid job: {e}")))?
    } else if let Some(raw) = p.get("job_yaml").and_then(|v| v.as_str()) {
        parse_job_document(raw)?
    } else if let Some(raw) = p.get("document").and_then(|v| v.as_str()) {
        parse_job_document(raw)?
    } else {
        return Err(ErrorObject::invalid_params(
            "job.submit needs params.job, params.job_yaml, or params.document",
        ));
    };
    let priority = p
        .get("priority")
        .and_then(|v| v.as_str())
        .and_then(Priority::parse)
        .unwrap_or_default();
    let restart = p
        .get("restart")
        .and_then(|v| v.as_str())
        .and_then(RestartPolicy::parse)
        .unwrap_or_default();
    engine
        .submit(job, priority, restart)
        .map(|rec| serde_json::to_value(rec).unwrap_or_default())
        .map_err(|e| ErrorObject::application(e.to_string()))
}

fn job_logs(engine: &Engine, params: Option<&Value>) -> Result<Value, ErrorObject> {
    let p = params.ok_or_else(|| ErrorObject::invalid_params("params required"))?;
    let id = p
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorObject::invalid_params("params.id required"))?;
    let stderr = p
        .get("stderr")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    engine
        .logs(id, stderr)
        .map(|t| json!({"text": t}))
        .map_err(|e| ErrorObject::application(e.to_string()))
}

fn job_id_op<F>(engine: &Engine, params: Option<&Value>, f: F) -> Result<Value, ErrorObject>
where
    F: FnOnce(&Engine, &str) -> Result<Value, crate::engine::EngineError>,
{
    let id = params
        .and_then(|p| p.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorObject::invalid_params("params.id required"))?;
    f(engine, id).map_err(|e| ErrorObject::application(e.to_string()))
}

fn subscribe(params: Option<&Value>) -> Result<Value, ErrorObject> {
    // Wire contract: client may subscribe; push notifications land in a later increment.
    let job_id = params
        .and_then(|p| p.get("job_id").or_else(|| p.get("id")))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(json!({
        "subscribed": true,
        "job_id": job_id,
        "push": false,
    }))
}

fn parse_job_document(raw: &str) -> Result<Job, ErrorObject> {
    let trimmed = raw.trim_start();
    if trimmed.starts_with('{') {
        serde_json::from_str(trimmed).map_err(|e| ErrorObject::invalid_params(e.to_string()))
    } else {
        serde_yaml::from_str(raw).map_err(|e| ErrorObject::invalid_params(e.to_string()))
    }
}

fn queue_view(jobs: &[crate::store::JobRecord]) -> Value {
    use crate::store::JobStatus;
    let mut queued = 0;
    let mut waiting_resources = 0;
    let mut running = 0;
    let mut completed = 0;
    let mut failed = 0;
    let mut cancelled = 0;
    for j in jobs {
        match j.status {
            JobStatus::Queued | JobStatus::Submitted | JobStatus::Scheduled => queued += 1,
            JobStatus::WaitingResources => waiting_resources += 1,
            JobStatus::Running => running += 1,
            JobStatus::Completed => completed += 1,
            JobStatus::Failed => failed += 1,
            JobStatus::Cancelled => cancelled += 1,
        }
    }
    json!({
        "queued": queued,
        "waiting_resources": waiting_resources,
        "running": running,
        "completed": completed,
        "failed": failed,
        "cancelled": cancelled,
        "jobs": jobs,
    })
}
