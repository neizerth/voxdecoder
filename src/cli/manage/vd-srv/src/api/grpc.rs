//! Optional gRPC transport for the Runtime API (ADR 0007).
//!
//! Disabled by default. Every service maps to the same Engine / dispatch semantics
//! as HTTP and JSON-RPC. `OperatorService::Health` is required.

use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::engine::Engine;
use crate::store::JobStatus;

use super::dispatch;
use super::rpc::{Id, Request as RpcRequest, Response as RpcResponse};

pub mod pb {
    tonic::include_proto!("voxdecoder.runtime.v1");
}

use pb::execution_service_server::{ExecutionService, ExecutionServiceServer};
use pb::event_service_server::{EventService, EventServiceServer};
use pb::operator_service_server::{OperatorService, OperatorServiceServer};
use pb::planning_service_server::{PlanningService, PlanningServiceServer};
use pb::{Empty, JobId, JsonBody};

#[derive(Clone)]
struct GrpcState {
    engine: Engine,
    stop: Arc<AtomicBool>,
}

/// Listen for gRPC on `bind` until `stop`.
pub fn serve(bind: &str, engine: Engine, stop: Arc<AtomicBool>) -> Result<(), String> {
    let addr: SocketAddr = bind
        .parse()
        .map_err(|e| format!("invalid grpc bind {bind}: {e}"))?;
    let state = GrpcState {
        engine,
        stop: Arc::clone(&stop),
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    rt.block_on(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| e.to_string())?;
        let local = listener.local_addr().map_err(|e| e.to_string())?;
        eprintln!("vd-srv gRPC listening on grpc://{local}");

        let stop_flag = Arc::clone(&state.stop);
        let incoming = TcpListenerStream::new(listener);
        tonic::transport::Server::builder()
            .add_service(OperatorServiceServer::new(state.clone()))
            .add_service(PlanningServiceServer::new(state.clone()))
            .add_service(ExecutionServiceServer::new(state.clone()))
            .add_service(EventServiceServer::new(state.clone()))
            .serve_with_incoming_shutdown(incoming, async move {
                while !stop_flag.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .map_err(|e| e.to_string())
    })
}

fn rpc_json(engine: &Engine, method: &str, params: Option<Value>) -> Result<Value, Status> {
    let req = RpcRequest::call(Id::number(1), method, params);
    match dispatch::handle(engine, req) {
        Some(RpcResponse {
            result: Some(v), ..
        }) => Ok(v),
        Some(RpcResponse {
            error: Some(err), ..
        }) => Err(Status::internal(err.message)),
        Some(_) => Err(Status::internal("empty response")),
        None => Err(Status::internal("notification-only")),
    }
}

fn json_ok(v: Value) -> Result<Response<JsonBody>, Status> {
    Ok(Response::new(JsonBody {
        json: serde_json::to_string(&v).unwrap_or_else(|_| "{}".into()),
    }))
}

fn parse_body(body: &JsonBody) -> Result<Value, Status> {
    if body.json.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&body.json).map_err(|e| Status::invalid_argument(e.to_string()))
}

#[tonic::async_trait]
impl OperatorService for GrpcState {
    async fn health(&self, _: Request<Empty>) -> Result<Response<JsonBody>, Status> {
        json_ok(self.engine.health_json())
    }

    async fn ready(&self, _: Request<Empty>) -> Result<Response<JsonBody>, Status> {
        json_ok(self.engine.health_json())
    }

    async fn live(&self, _: Request<Empty>) -> Result<Response<JsonBody>, Status> {
        json_ok(json!({"ok": true}))
    }

    async fn doctor(&self, _: Request<Empty>) -> Result<Response<JsonBody>, Status> {
        let health = self.engine.health_json();
        let info = rpc_json(&self.engine, "server.info", None)?;
        json_ok(json!({"health": health, "server_info": info}))
    }

    async fn server_info(&self, _: Request<Empty>) -> Result<Response<JsonBody>, Status> {
        json_ok(rpc_json(&self.engine, "server.info", None)?)
    }
}

#[tonic::async_trait]
impl PlanningService for GrpcState {
    async fn plan_audio(&self, req: Request<JsonBody>) -> Result<Response<JsonBody>, Status> {
        let params = parse_body(req.get_ref())?;
        json_ok(rpc_json(&self.engine, "plan.audio", Some(params))?)
    }

    async fn plan_meeting(&self, req: Request<JsonBody>) -> Result<Response<JsonBody>, Status> {
        let params = parse_body(req.get_ref())?;
        json_ok(rpc_json(&self.engine, "plan.meeting", Some(params))?)
    }
}

#[tonic::async_trait]
impl ExecutionService for GrpcState {
    async fn submit_job(&self, req: Request<JsonBody>) -> Result<Response<JsonBody>, Status> {
        let params = parse_body(req.get_ref())?;
        json_ok(rpc_json(&self.engine, "job.submit", Some(params))?)
    }

    async fn list_jobs(&self, _: Request<Empty>) -> Result<Response<JsonBody>, Status> {
        json_ok(rpc_json(&self.engine, "job.list", None)?)
    }

    async fn get_job(&self, req: Request<JobId>) -> Result<Response<JsonBody>, Status> {
        let id = req.into_inner().id;
        json_ok(rpc_json(
            &self.engine,
            "job.status",
            Some(json!({"id": id})),
        )?)
    }

    async fn cancel_job(&self, req: Request<JobId>) -> Result<Response<JsonBody>, Status> {
        let id = req.into_inner().id;
        json_ok(rpc_json(
            &self.engine,
            "job.cancel",
            Some(json!({"id": id})),
        )?)
    }
}

type EventStream = Pin<Box<dyn Stream<Item = Result<JsonBody, Status>> + Send>>;

#[tonic::async_trait]
impl EventService for GrpcState {
    type WatchJobStream = EventStream;

    async fn watch_job(
        &self,
        req: Request<JobId>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        let id = req.into_inner().id;
        if id.is_empty() {
            return Err(Status::invalid_argument("id required"));
        }
        let _ = self
            .engine
            .job(&id)
            .map_err(|e| Status::not_found(e.to_string()))?;

        let engine = self.engine.clone();
        let stop = Arc::clone(&self.stop);
        let (tx, rx) = mpsc::channel(64);

        thread::spawn(move || {
            let mut sent = 0usize;
            loop {
                if stop.load(Ordering::SeqCst) || engine.is_stopped() {
                    break;
                }
                let Ok(events) = engine.events(&id) else {
                    break;
                };
                while sent < events.len() {
                    let ev = &events[sent];
                    let body = JsonBody {
                        json: serde_json::to_string(ev).unwrap_or_else(|_| "{}".into()),
                    };
                    if tx.blocking_send(Ok(body)).is_err() {
                        return;
                    }
                    let kind = ev.kind.as_str();
                    sent += 1;
                    if matches!(
                        kind,
                        "JobCompleted" | "JobFinished" | "JobFailed" | "JobCancelled"
                    ) {
                        return;
                    }
                }
                if let Ok(rec) = engine.job(&id) {
                    if matches!(
                        rec.status,
                        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
                    ) && sent >= events.len()
                    {
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(300));
            }
        });

        let stream: EventStream = Box::pin(ReceiverStream::new(rx));
        Ok(Response::new(stream))
    }
}
