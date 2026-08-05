//! gRPC transport (ADR 0007) — typed Health / GetJob / WatchJob.

#![allow(clippy::default_trait_access, clippy::field_reassign_with_default)]

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;
use tokio_stream::StreamExt;
use vd_pipeline::{Capability, Job, JobInput, Step};
use vd_srv::api::grpc;
use vd_srv::api::grpc::pb::event::Payload;
use vd_srv::api::grpc::pb::execution_service_client::ExecutionServiceClient;
use vd_srv::api::grpc::pb::event_service_client::EventServiceClient;
use vd_srv::api::grpc::pb::operator_service_client::OperatorServiceClient;
use vd_srv::api::grpc::pb::{Empty, JobId, JobStatus};
use vd_srv::config::ServerConfig;
use vd_srv::engine::{TransportEndpoint, TransportStatus};
use vd_srv::store::{Priority, RestartPolicy};
use vd_srv::Engine;

fn sample_job(dir: &std::path::Path) -> Job {
    let sample = dir.join("sample.txt");
    std::fs::write(&sample, "hello world\n").unwrap();
    Job {
        version: 1,
        id: None,
        name: Some("fix".into()),
        working_dir: Some(dir.to_path_buf()),
        input: JobInput::default(),
        context: Default::default(),
        output: Default::default(),
        continue_on_error: false,
        max_parallel: Some(1),
        resources: Default::default(),
        steps: vec![{
            let mut s = Step::new(Capability::FixCasing);
            s.input = Some(sample.display().to_string());
            s.options
                .insert("overwrite".into(), vd_pipeline::ArgValue::Bool(true));
            s.into()
        }],
    }
}

fn start_grpc(engine: &Engine) -> (String, Arc<AtomicBool>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let bind = addr.to_string();
    engine.set_transports(TransportStatus {
        grpc: Some(TransportEndpoint::on(format!("grpc://{bind}"))),
        http: Some(TransportEndpoint::off()),
        ..Default::default()
    });
    let stop = Arc::new(AtomicBool::new(false));
    let serve_engine = engine.clone();
    let serve_stop = Arc::clone(&stop);
    let handle = thread::spawn(move || {
        let _ = grpc::serve(&bind, serve_engine, serve_stop);
    });
    (format!("http://{addr}"), stop, handle)
}

#[test]
fn grpc_transport_health_typed() {
    let dir = TempDir::new().unwrap();
    let mut cfg = ServerConfig::default();
    cfg.workers = 1;
    let engine = Engine::start(dir.path().to_path_buf(), cfg).unwrap();
    let (endpoint, stop, handle) = start_grpc(&engine);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let health = rt.block_on(async {
        let mut last_err = String::new();
        for _ in 0..50 {
            let channel = match tonic::transport::Endpoint::from_shared(endpoint.clone()) {
                Ok(ep) => ep
                    .connect_timeout(Duration::from_millis(300))
                    .timeout(Duration::from_secs(2))
                    .connect_lazy(),
                Err(e) => {
                    last_err = e.to_string();
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    continue;
                }
            };
            let mut client = OperatorServiceClient::new(channel);
            match tokio::time::timeout(Duration::from_secs(2), client.health(Empty {})).await {
                Ok(Ok(resp)) => return resp.into_inner(),
                Ok(Err(e)) => last_err = e.to_string(),
                Err(_) => last_err = "health RPC timed out".into(),
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
        panic!("grpc health failed: {last_err}");
    });

    assert!(health.workers >= 1, "health={health:?}");
    assert!(!health.data_dir.is_empty());

    stop.store(true, Ordering::SeqCst);
    engine.stop();
    let _ = handle.join();
}

#[test]
fn grpc_get_job_and_watch_job_typed() {
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
    let (endpoint, stop, handle) = start_grpc(&engine);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let job_id = rec.id.clone();
    rt.block_on(async {
        let mut last_err = String::new();
        let channel = {
            let mut ch = None;
            for _ in 0..50 {
                match tonic::transport::Endpoint::from_shared(endpoint.clone()) {
                    Ok(ep) => {
                        ch = Some(
                            ep.connect_timeout(Duration::from_millis(300))
                                .timeout(Duration::from_secs(10))
                                .connect_lazy(),
                        );
                        break;
                    }
                    Err(e) => {
                        last_err = e.to_string();
                        tokio::time::sleep(Duration::from_millis(40)).await;
                    }
                }
            }
            ch.unwrap_or_else(|| panic!("connect failed: {last_err}"))
        };

        let mut exec = ExecutionServiceClient::new(channel.clone());
        let view = exec
            .get_job(JobId {
                id: job_id.clone(),
            })
            .await
            .expect("GetJob")
            .into_inner();
        assert_eq!(view.id, job_id);
        assert_ne!(view.status, JobStatus::Unspecified as i32);

        let mut events = EventServiceClient::new(channel);
        let mut stream = events
            .watch_job(JobId { id: job_id })
            .await
            .expect("WatchJob")
            .into_inner();

        let mut saw_terminal = false;
        let mut n = 0usize;
        while let Some(item) = stream.next().await {
            let ev = item.expect("event");
            n += 1;
            match ev.payload {
                Some(Payload::JobCompleted(_))
                | Some(Payload::JobFinished(_))
                | Some(Payload::JobFailed(_))
                | Some(Payload::JobCancelled(_)) => {
                    saw_terminal = true;
                    break;
                }
                _ => {}
            }
            if n > 200 {
                break;
            }
        }
        assert!(n > 0, "expected at least one event");
        assert!(saw_terminal, "expected terminal job event");
    });

    stop.store(true, Ordering::SeqCst);
    engine.stop();
    let _ = handle.join();
}
