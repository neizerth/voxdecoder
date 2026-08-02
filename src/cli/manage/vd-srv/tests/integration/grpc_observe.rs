//! gRPC transport (ADR 0007) — Health required on every transport.

#![allow(clippy::default_trait_access, clippy::field_reassign_with_default)]

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;
use vd_srv::api::grpc;
use vd_srv::api::grpc::pb::operator_service_client::OperatorServiceClient;
use vd_srv::api::grpc::pb::Empty;
use vd_srv::config::ServerConfig;
use vd_srv::engine::{TransportEndpoint, TransportStatus};
use vd_srv::Engine;

#[test]
fn grpc_transport_health() {
    let dir = TempDir::new().unwrap();
    let mut cfg = ServerConfig::default();
    cfg.workers = 1;
    let engine = Engine::start(dir.path().to_path_buf(), cfg).unwrap();

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
    let handle = thread::spawn(move || grpc::serve(&bind, serve_engine, serve_stop));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let endpoint = format!("http://{addr}");
    let body = rt.block_on(async {
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
                Ok(Ok(resp)) => {
                    return serde_json::from_str::<serde_json::Value>(&resp.into_inner().json)
                        .expect("health json");
                }
                Ok(Err(e)) => last_err = e.to_string(),
                Err(_) => last_err = "health RPC timed out".into(),
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
        panic!("grpc health failed: {last_err}");
    });

    assert!(body.get("workers").is_some(), "health={body}");

    stop.store(true, Ordering::SeqCst);
    engine.stop();
    let _ = handle.join();
}
