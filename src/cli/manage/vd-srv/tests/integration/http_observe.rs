//! HTTP transport (ADR 0006 / 0007) over a dedicated bind.

#![allow(clippy::default_trait_access, clippy::field_reassign_with_default)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use vd_pipeline::{Capability, Job, JobInput, Step};
use vd_srv::api::http;
use vd_srv::config::ServerConfig;
use vd_srv::engine::{TransportEndpoint, TransportStatus};
use vd_srv::store::{Priority, RestartPolicy};
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
        steps: vec![{
            let mut s = Step::new(Capability::FixCasing);
            s.input = Some(sample.display().to_string());
            s.options
                .insert("overwrite".into(), vd_pipeline::ArgValue::Bool(true));
            s.into()
        }],
    }
}

fn http_get(addr: &str, path: &str) -> (u16, String) {
    let mut stream = std::net::TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf);
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();
    (status, body)
}

#[test]
fn http_transport_health_openapi_and_job_status() {
    let dir = TempDir::new().unwrap();
    let mut cfg = ServerConfig::default();
    cfg.workers = 1;
    let engine = Engine::start(dir.path().to_path_buf(), cfg).unwrap();
    engine.set_transports(TransportStatus {
        http: Some(TransportEndpoint::on("http://127.0.0.1:0")),
        grpc: Some(TransportEndpoint::off()),
        ..Default::default()
    });
    let rec = engine
        .submit(
            sample_job(dir.path()),
            Priority::Normal,
            RestartPolicy::Resume,
        )
        .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let bind = addr.to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let serve_engine = engine.clone();
    let serve_stop = Arc::clone(&stop);
    let handle = thread::spawn(move || http::serve(&bind, serve_engine, serve_stop));

    for _ in 0..50 {
        if std::net::TcpStream::connect(addr).is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let addr_s = addr.to_string();
    let (st, body) = http_get(&addr_s, "/health");
    assert_eq!(st, 200, "health body={body}");
    let health: Value = serde_json::from_str(&body).unwrap();
    assert!(health.get("workers").is_some());

    let (st, body) = http_get(&addr_s, "/live");
    assert_eq!(st, 200);
    assert!(body.contains("\"ok\":true"));

    let (st, body) = http_get(&addr_s, "/server_info");
    assert_eq!(st, 200, "server_info={body}");
    let info: Value = serde_json::from_str(&body).unwrap();
    assert!(info["runtime"]["api_version"].is_string());
    assert_eq!(info["transports"]["http"]["enabled"], true);
    assert!(info["health"].get("workers").is_some());

    let (st, body) = http_get(&addr_s, "/openapi.json");
    assert_eq!(st, 200, "openapi={body}");
    let oa: Value = serde_json::from_str(&body).unwrap();
    assert!(oa["paths"]["/health"].is_object());

    let (st, body) = http_get(&addr_s, "/docs");
    assert_eq!(st, 200);
    assert!(body.contains("openapi.json"));

    let (st, body) = http_get(&addr_s, &format!("/jobs/{}", rec.id));
    assert_eq!(st, 200, "job body={body}");
    let job: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(job["id"], rec.id);

    let (st, body) = http_get(&addr_s, &format!("/jobs/{}/events", rec.id));
    assert_eq!(st, 200, "events={body}");
    assert!(body.contains("event:") || body.contains("Job"));

    let (st, _) = http_get(&addr_s, "/jobs/does-not-exist");
    assert_eq!(st, 404);

    stop.store(true, Ordering::SeqCst);
    engine.stop();
    let _ = handle.join();
}
