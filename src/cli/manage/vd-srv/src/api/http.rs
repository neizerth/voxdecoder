//! Optional HTTP transport for the Runtime API (ADR 0006 / 0007).
//!
//! Thin REST/SSE adapter — every route forwards to Planning / Execution / Operator
//! via the same JSON-RPC dispatch used by UDS/TCP. Disabled unless `--http` / config.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use crate::engine::Engine;
use crate::store::JobStatus;

use super::dispatch;
use super::openapi;
use super::rpc::{code, Id, Request, Response};

/// Listen for HTTP on `bind` until `stop`. One request per connection (curl-friendly).
pub fn serve(bind: &str, engine: Engine, stop: Arc<AtomicBool>) -> Result<(), String> {
    let addr: SocketAddr = bind
        .parse()
        .map_err(|e| format!("invalid http bind {bind}: {e}"))?;
    let listener = TcpListener::bind(addr).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    eprintln!("vd-srv HTTP listening on http://{addr}");

    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let eng = engine.clone();
                let stop_c = Arc::clone(&stop);
                thread::spawn(move || {
                    let _ = handle_connection(stream, &eng, &stop_c);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

fn handle_connection(
    stream: TcpStream,
    engine: &Engine,
    stop: &AtomicBool,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .ok();
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;

    let mut request_line = String::new();
    let n = reader.read_line(&mut request_line).map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(());
    }
    let (method, path) = parse_request_line(request_line.trim())?;
    let headers = read_headers(&mut reader)?;
    let body = read_body(&mut reader, &headers)?;

    if path.starts_with("/jobs/") && path.ends_with("/events") && method == "GET" {
        let id = path
            .trim_start_matches("/jobs/")
            .trim_end_matches("/events")
            .trim_matches('/');
        return write_sse_events(&mut writer, engine, id, stop);
    }

    if method == "GET" && path == "/docs" {
        return write_html(&mut writer, 200, &openapi::docs_html());
    }
    if method == "GET" && path == "/openapi.yaml" {
        let yaml = openapi::yaml()?;
        return write_raw(
            &mut writer,
            200,
            "application/yaml",
            yaml.as_bytes(),
        );
    }
    if method == "GET" && path == "/openapi.json" {
        let raw = serde_json::to_string_pretty(&openapi::document()).map_err(|e| e.to_string())?;
        return write_raw(
            &mut writer,
            200,
            "application/json",
            raw.as_bytes(),
        );
    }

    let (status, payload) = route(engine, &method, &path, body);
    write_json(&mut writer, status, &payload)
}

fn route(engine: &Engine, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    match (method, path) {
        ("GET", "/live") => (200, json!({"ok": true})),
        ("GET", "/ready") | ("GET", "/health") => match call(engine, "server.health", None) {
            Ok(v) => (200, v),
            Err((c, e)) => (c, json!({"error": e})),
        },
        ("GET", "/doctor") => match doctor(engine) {
            Ok(v) => (200, v),
            Err((c, e)) => (c, json!({"error": e})),
        },
        ("GET", "/server_info") => match call(engine, "server.info", None) {
            Ok(v) => (200, v),
            Err((c, e)) => (c, json!({"error": e})),
        },
        ("POST", "/planning/audio") => {
            forward(engine, "plan.audio", body.or_else(|| Some(json!({}))))
        }
        ("POST", "/planning/meeting") => {
            forward(engine, "plan.meeting", body.or_else(|| Some(json!({}))))
        }
        ("POST", "/jobs") => forward(engine, "job.submit", body.or_else(|| Some(json!({})))),
        ("GET", "/jobs") => match call(engine, "job.list", None) {
            Ok(v) => (200, v),
            Err((c, e)) => (c, json!({"error": e})),
        },
        ("POST", p) => {
            if let Some(id) = job_action_id(p, "cancel") {
                forward(engine, "job.cancel", Some(json!({"id": id})))
            } else {
                (404, json!({"error": "not found"}))
            }
        }
        ("GET", p) => {
            if let Some(id) = job_id_path(p) {
                forward(engine, "job.status", Some(json!({"id": id})))
            } else {
                (404, json!({"error": "not found"}))
            }
        }
        _ => (404, json!({"error": "not found"})),
    }
}

/// `/jobs/:id` → Some(id)
fn job_id_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/jobs/")?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(rest.to_string())
}

/// `/jobs/:id/cancel` → Some(id)
fn job_action_id(path: &str, action: &str) -> Option<String> {
    let rest = path.strip_prefix("/jobs/")?;
    let (id, act) = rest.rsplit_once('/')?;
    if act != action || id.is_empty() || id.contains('/') {
        return None;
    }
    Some(id.to_string())
}

fn forward(engine: &Engine, method: &str, params: Option<Value>) -> (u16, Value) {
    match call(engine, method, params) {
        Ok(v) => (200, v),
        Err((c, e)) => (c, json!({"error": e})),
    }
}

fn doctor(engine: &Engine) -> Result<Value, (u16, String)> {
    let health = call(engine, "server.health", None)?;
    let info = call(engine, "server.info", None)?;
    Ok(json!({"health": health, "server_info": info}))
}

fn call(engine: &Engine, method: &str, params: Option<Value>) -> Result<Value, (u16, String)> {
    let req = Request::call(Id::number(1), method, params);
    match dispatch::handle(engine, req) {
        Some(Response {
            result: Some(v), ..
        }) => Ok(v),
        Some(Response {
            error: Some(err), ..
        }) => Err((http_status_for_rpc(err.code, &err.message), err.message)),
        Some(_) => Err((500, "empty response".into())),
        None => Err((500, "notification-only".into())),
    }
}

fn http_status_for_rpc(rpc_code: i64, message: &str) -> u16 {
    if message.contains("not found") {
        return 404;
    }
    match rpc_code {
        code::INVALID_PARAMS | code::INVALID_REQUEST => 400,
        code::METHOD_NOT_FOUND => 404,
        _ => 500,
    }
}

fn write_sse_events(
    writer: &mut impl Write,
    engine: &Engine,
    id: &str,
    stop: &AtomicBool,
) -> Result<(), String> {
    if id.is_empty() {
        return write_json(writer, 404, &json!({"error": "not found"}));
    }
    // Ensure job exists.
    if let Err(e) = engine.job(id) {
        let msg = e.to_string();
        let status = if msg.contains("not found") { 404 } else { 500 };
        return write_json(writer, status, &json!({"error": msg}));
    }

    write!(
        writer,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n"
    )
    .map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;

    let mut sent = 0usize;
    loop {
        if stop.load(Ordering::SeqCst) || engine.is_stopped() {
            break;
        }
        let events = match engine.events(id) {
            Ok(e) => e,
            Err(_) => break,
        };
        while sent < events.len() {
            let ev = &events[sent];
            let kind = ev.kind.clone();
            let data = serde_json::to_string(ev).unwrap_or_else(|_| "{}".into());
            if write!(writer, "event: {kind}\ndata: {data}\n\n").is_err() {
                return Ok(());
            }
            sent += 1;
            if matches!(
                kind.as_str(),
                "JobCompleted" | "JobFinished" | "JobFailed" | "JobCancelled"
            ) {
                let _ = writer.flush();
                return Ok(());
            }
        }
        if writer.flush().is_err() {
            return Ok(());
        }
        if let Ok(rec) = engine.job(id) {
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
    Ok(())
}

fn parse_request_line(line: &str) -> Result<(String, String), String> {
    let mut parts = line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "invalid HTTP request line".to_string())?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| "invalid HTTP request line".to_string())?;
    let path = target.split('?').next().unwrap_or(target).to_string();
    Ok((method, path))
}

fn read_headers(reader: &mut impl BufRead) -> Result<Vec<(String, String)>, String> {
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    Ok(headers)
}

fn read_body(
    reader: &mut impl BufRead,
    headers: &[(String, String)],
) -> Result<Option<Value>, String> {
    let len = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    if len == 0 {
        return Ok(None);
    }
    let mut buf = vec![0_u8; len];
    reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
    if buf.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(None);
    }
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| format!("invalid JSON body: {e}"))
}

fn write_json(writer: &mut impl Write, status: u16, body: &Value) -> Result<(), String> {
    let raw = serde_json::to_string(body).map_err(|e| e.to_string())?;
    write_raw(writer, status, "application/json", raw.as_bytes())
}

fn write_html(writer: &mut impl Write, status: u16, body: &str) -> Result<(), String> {
    write_raw(writer, status, "text/html; charset=utf-8", body.as_bytes())
}

fn write_raw(
    writer: &mut impl Write,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    write!(
        writer,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .map_err(|e| e.to_string())?;
    writer.write_all(body).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_request_line;

    #[test]
    fn parses_get_path() {
        let (m, p) = parse_request_line("GET /jobs/abc?x=1 HTTP/1.1").unwrap();
        assert_eq!(m, "GET");
        assert_eq!(p, "/jobs/abc");
    }
}
