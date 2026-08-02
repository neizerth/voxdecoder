//! Local control socket (newline-delimited JSON).

use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use vd_pipeline::Job;

use crate::engine::Engine;
use crate::store::{Priority, RestartPolicy};

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Ping,
    Stop,
    Submit {
        #[serde(default)]
        job: Option<Job>,
        #[serde(default)]
        job_yaml: Option<String>,
        #[serde(default)]
        priority: Option<String>,
        #[serde(default)]
        restart: Option<String>,
    },
    Queue,
    Jobs,
    JobInfo {
        id: String,
    },
    Events {
        id: String,
    },
    Artifacts {
        id: String,
    },
    Logs {
        id: String,
        #[serde(default)]
        stderr: bool,
    },
    Cancel {
        id: String,
    },
    Health,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Response {
    fn ok(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            error: None,
            data: Some(data),
        }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
            data: None,
        }
    }
}

pub fn serve_socket(
    socket: &Path,
    engine: Engine,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    if socket.exists() {
        std::fs::remove_file(socket).map_err(|e| e.to_string())?;
    }
    if let Some(parent) = socket.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let listener = UnixListener::bind(socket).map_err(|e| e.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;

    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let eng = engine.clone();
                thread::spawn(move || {
                    let _ = handle_client(stream, &eng);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let _ = std::fs::remove_file(socket);
    Ok(())
}

fn handle_client(stream: UnixStream, engine: &Engine) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    if line.trim().is_empty() {
        return Ok(());
    }
    let req: Request = serde_json::from_str(line.trim()).map_err(|e| e.to_string())?;
    let resp = dispatch(engine, req);
    let out = serde_json::to_string(&resp).map_err(|e| e.to_string())?;
    writeln!(writer, "{out}").map_err(|e| e.to_string())?;
    let _ = writer.shutdown(Shutdown::Both);
    Ok(())
}

fn dispatch(engine: &Engine, req: Request) -> Response {
    match req {
        Request::Ping => Response::ok(serde_json::json!({"pong": true})),
        Request::Stop => {
            engine.stop();
            Response::ok(serde_json::json!({"stopping": true}))
        }
        Request::Submit {
            job,
            job_yaml,
            priority,
            restart,
        } => match submit(engine, job, job_yaml, priority, restart) {
            Ok(rec) => Response::ok(serde_json::to_value(rec).unwrap_or_default()),
            Err(e) => Response::err(e),
        },
        Request::Queue | Request::Jobs => match engine.list() {
            Ok(jobs) => Response::ok(serde_json::to_value(queue_view(&jobs)).unwrap_or_default()),
            Err(e) => Response::err(e.to_string()),
        },
        Request::JobInfo { id } => match engine.job(&id) {
            Ok(j) => Response::ok(serde_json::to_value(j).unwrap_or_default()),
            Err(e) => Response::err(e.to_string()),
        },
        Request::Events { id } => match engine.events(&id) {
            Ok(ev) => Response::ok(serde_json::to_value(ev).unwrap_or_default()),
            Err(e) => Response::err(e.to_string()),
        },
        Request::Artifacts { id } => match engine.artifacts(&id) {
            Ok(a) => Response::ok(serde_json::to_value(a).unwrap_or_default()),
            Err(e) => Response::err(e.to_string()),
        },
        Request::Logs { id, stderr } => match engine.logs(&id, stderr) {
            Ok(t) => Response::ok(serde_json::json!({"text": t})),
            Err(e) => Response::err(e.to_string()),
        },
        Request::Cancel { id } => match engine.cancel(&id) {
            Ok(j) => Response::ok(serde_json::to_value(j).unwrap_or_default()),
            Err(e) => Response::err(e.to_string()),
        },
        Request::Health => {
            let busy = engine.workers_busy();
            let total = engine.workers_total();
            let resources = engine.resources_snapshot();
            Response::ok(serde_json::json!({
                "workers_busy": busy,
                "workers": total,
                "resources": resources,
                "data_dir": engine.data_dir(),
            }))
        }
    }
}

fn submit(
    engine: &Engine,
    job: Option<Job>,
    job_yaml: Option<String>,
    priority: Option<String>,
    restart: Option<String>,
) -> Result<crate::store::JobRecord, String> {
    let job = if let Some(j) = job {
        j
    } else if let Some(raw) = job_yaml {
        parse_job_document(&raw)?
    } else {
        return Err("submit needs job or job_yaml".into());
    };
    let priority = priority
        .as_deref()
        .and_then(Priority::parse)
        .unwrap_or_default();
    let restart = restart
        .as_deref()
        .and_then(RestartPolicy::parse)
        .unwrap_or_default();
    engine
        .submit(job, priority, restart)
        .map_err(|e| e.to_string())
}

fn parse_job_document(raw: &str) -> Result<Job, String> {
    let trimmed = raw.trim_start();
    if trimmed.starts_with('{') {
        serde_json::from_str(trimmed).map_err(|e| e.to_string())
    } else {
        serde_yaml::from_str(raw).map_err(|e| e.to_string())
    }
}

fn queue_view(jobs: &[crate::store::JobRecord]) -> serde_json::Value {
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
    serde_json::json!({
        "queued": queued,
        "waiting_resources": waiting_resources,
        "running": running,
        "completed": completed,
        "failed": failed,
        "cancelled": cancelled,
        "jobs": jobs,
    })
}

pub fn call(socket: &Path, req: &Request) -> Result<Response, String> {
    let mut stream = UnixStream::connect(socket).map_err(|e| {
        format!(
            "cannot connect to {}: {e} (is vd-srv serve running?)",
            socket.display()
        )
    })?;
    let body = serde_json::to_string(req).map_err(|e| e.to_string())?;
    writeln!(stream, "{body}").map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    serde_json::from_str(line.trim()).map_err(|e| e.to_string())
}

// Re-export Request construction helpers need Serialize on Request for client —
impl Serialize for Request {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Manual via Value for tagged enum client sends.
        let v = match self {
            Self::Ping => serde_json::json!({"op":"ping"}),
            Self::Stop => serde_json::json!({"op":"stop"}),
            Self::Submit {
                job,
                job_yaml,
                priority,
                restart,
            } => {
                let mut m = serde_json::Map::new();
                m.insert("op".into(), serde_json::json!("submit"));
                if let Some(j) = job {
                    m.insert(
                        "job".into(),
                        serde_json::to_value(j).unwrap_or(serde_json::Value::Null),
                    );
                }
                if let Some(y) = job_yaml {
                    m.insert("job_yaml".into(), serde_json::json!(y));
                }
                if let Some(p) = priority {
                    m.insert("priority".into(), serde_json::json!(p));
                }
                if let Some(r) = restart {
                    m.insert("restart".into(), serde_json::json!(r));
                }
                serde_json::Value::Object(m)
            }
            Self::Queue => serde_json::json!({"op":"queue"}),
            Self::Jobs => serde_json::json!({"op":"jobs"}),
            Self::JobInfo { id } => serde_json::json!({"op":"job_info","id":id}),
            Self::Events { id } => serde_json::json!({"op":"events","id":id}),
            Self::Artifacts { id } => serde_json::json!({"op":"artifacts","id":id}),
            Self::Logs { id, stderr } => {
                serde_json::json!({"op":"logs","id":id,"stderr":stderr})
            }
            Self::Cancel { id } => serde_json::json!({"op":"cancel","id":id}),
            Self::Health => serde_json::json!({"op":"health"}),
        };
        v.serialize(serializer)
    }
}
