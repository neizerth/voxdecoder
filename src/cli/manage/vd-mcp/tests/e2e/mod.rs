//! End-to-end MCP gateway against a live Runtime.

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use tempfile::TempDir;
use vd_mcp::client::RuntimeClient;
use vd_mcp::mcp::protocol;
use vd_srv::api::{self, Endpoint};
use vd_srv::config::ServerConfig;
use vd_srv::Engine;

#[test]
fn initialize_and_process_audio_plan_only() {
    let dir = TempDir::new().unwrap();
    let data = dir.path().join("data");
    let sock = dir.path().join("srv.sock");
    fs::create_dir_all(&data).unwrap();

    let mut cfg = ServerConfig::default();
    cfg.workers = 1;
    let engine = Engine::start(data, cfg).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let endpoint = Endpoint::Uds(sock.clone());
    let serve_engine = engine.clone();
    let serve_stop = Arc::clone(&stop);
    let serve_ep = endpoint.clone();
    let handle = thread::spawn(move || api::serve(&serve_ep, serve_engine, serve_stop));

    wait_for_socket(&sock);

    let client = RuntimeClient::new(endpoint);
    let init = protocol::handle(
        &client,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "vd-mcp-e2e", "version": "0"}
            }
        }),
    )
    .unwrap();
    assert_eq!(
        init.pointer("/result/serverInfo/name")
            .and_then(Value::as_str),
        Some("vd-mcp")
    );

    let tools = protocol::handle(
        &client,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    )
    .unwrap();
    let names: Vec<&str> = tools
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    assert!(names.contains(&"process_audio"));
    assert!(names.contains(&"process_meeting"));

    let call = protocol::handle(
        &client,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "process_audio",
                "arguments": {
                    "audio": {"path": "/tmp/e2e-audio.wav"},
                    "execute": false
                }
            }
        }),
    )
    .unwrap();
    let text = call
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap();
    let payload: Value = serde_json::from_str(text).unwrap();
    assert!(
        payload.get("job").is_some(),
        "expected plan-only job: {payload}"
    );
    assert!(
        payload.get("id").is_none(),
        "execute:false must not submit"
    );

    let meeting = protocol::handle(
        &client,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "process_meeting",
                "arguments": {
                    "audio": {"path": "/tmp/e2e-meeting.wav"},
                    "working_dir": "/work",
                    "execute": false
                }
            }
        }),
    )
    .unwrap();
    let meeting_text = meeting
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap();
    let meeting_payload: Value = serde_json::from_str(meeting_text).unwrap();
    assert!(
        meeting_payload.get("job").is_some(),
        "expected meeting plan: {meeting_payload}"
    );

    stop.store(true, Ordering::SeqCst);
    engine.stop();
    let _ = handle.join();
}

fn wait_for_socket(sock: &std::path::Path) {
    for _ in 0..50 {
        if sock.exists() {
            thread::sleep(Duration::from_millis(50));
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("runtime socket never appeared: {}", sock.display());
}
