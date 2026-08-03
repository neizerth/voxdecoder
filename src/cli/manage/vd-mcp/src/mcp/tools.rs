//! MCP tool registry and Runtime API forwarding.

use serde_json::{json, Value};

use crate::client::RuntimeClient;

pub fn list() -> Vec<Value> {
    vec![
        tool(
            "process_audio",
            "Plan and optionally execute an audio processing Job. On macOS ASR defaults to Metal. Optional speed (e.g. 2.0–2.2) shortens wall time via preprocess; timestamps stay correct. When execute=true, response includes id and observe hints.",
            audio_schema(),
        ),
        tool(
            "process_meeting",
            "Plan and optionally execute a meeting Job. When execute=true, response includes id and observe hints for status polling.",
            meeting_schema(),
        ),
        tool(
            "submit_job",
            "Submit a complete Runtime Job document.",
            json!({"type":"object","properties":{"job":{},"job_yaml":{"type":"string"},"document":{"type":"string"}}}),
        ),
        tool(
            "get_job",
            "Poll Job status by id. Response includes status, progress (0–100), and phase when available.",
            id_schema(),
        ),
        tool("cancel_job", "Cancel a Job by id.", id_schema()),
        tool("list_jobs", "List Runtime Jobs.", json!({"type":"object"})),
        tool("list_artifacts", "List artifacts for a Job.", id_schema()),
        tool("health", "Get Runtime health.", json!({"type":"object"})),
        tool(
            "doctor",
            "Get Runtime health and discovery information.",
            json!({"type":"object"}),
        ),
        tool(
            "server_info",
            "Get Runtime API discovery information.",
            json!({"type":"object"}),
        ),
    ]
}

pub fn call(client: &RuntimeClient, name: &str, arguments: Value) -> Result<Value, String> {
    match name {
        "process_audio" => with_observe(client.call("plan.audio", Some(arguments))?),
        "process_meeting" => with_observe(client.call("plan.meeting", Some(arguments))?),
        "submit_job" => with_observe(client.call("job.submit", Some(arguments))?),
        "get_job" => client.call("job.status", Some(arguments)),
        "cancel_job" => client.call("job.cancel", Some(arguments)),
        "list_jobs" => client.call("job.list", Some(arguments)),
        "list_artifacts" => client.call("artifact.list", Some(arguments)),
        "health" => client.call("server.health", Some(arguments)),
        "doctor" => {
            let health = client.call("server.health", None)?;
            let info = client.call("server.info", None)?;
            Ok(json!({"health": health, "server_info": info}))
        }
        "server_info" => client.call("server.info", Some(arguments)),
        _ => Err(format!("unknown tool: {name}")),
    }
}

/// Attach poll hints when a Job id is present (MCP-side; Runtime unchanged).
fn with_observe(mut value: Value) -> Result<Value, String> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(id) = id {
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "observe".into(),
                json!({
                    "mcp_tool": "get_job",
                    "cli": format!(
                        "vdctl api job.status --params {{\"id\":\"{id}\"}} --json"
                    ),
                    "rule": "Poll with MCP get_job until completed|failed|cancelled. Report progress and phase from the response when present. Do not use curl/HTTP when MCP tools are available.",
                }),
            );
        }
    }
    Ok(value)
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name": name, "description": description, "inputSchema": input_schema})
}

#[cfg(test)]
mod observe_tests {
    use super::with_observe;
    use serde_json::json;

    #[test]
    fn attaches_observe_when_id_present() {
        let out = with_observe(json!({"id": "job-1", "status": "running"})).unwrap();
        assert_eq!(out["observe"]["mcp_tool"], "get_job");
        assert!(out["observe"]["cli"].as_str().unwrap().contains("job-1"));
        assert!(out["observe"]["rule"]
            .as_str()
            .unwrap()
            .contains("get_job"));
    }

    #[test]
    fn skips_observe_without_id() {
        let out = with_observe(json!({"job": {"version": 1}})).unwrap();
        assert!(out.get("observe").is_none());
    }
}

fn id_schema() -> Value {
    json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"}}})
}

fn input_source_schema() -> Value {
    json!({
        "type": "object",
        "description": "Exactly one of path, uri, url, artifact, or blob.",
        "properties": {
            "path": {"type":"string"},
            "uri": {"type":"string"},
            "url": {"type":"string","description":"Online media (http/https); Planning resolves via vd-input / vd-url before Job build"},
            "artifact": {"type":"string"},
            "blob": {"type":"string"}
        }
    })
}

fn audio_schema() -> Value {
    json!({
        "type": "object",
        "required": ["audio"],
        "properties": {
            "audio": input_source_schema(),
            "execute": {"type":"boolean","default":true},
            "run": {"type":"boolean","description":"Alias for execute"},
            "engine": {"type":"string","enum":["gigaam","whisper"]},
            "model": {"type":"string"},
            "device": {"type":"string","description":"ASR device. On macOS defaults to metal when omitted."},
            "speed": {
                "type":"number",
                "minimum": 0.25,
                "maximum": 4.0,
                "description":"Preprocess playback speed (e.g. 1.5, 2, 2.2). Speeds up ASR; timestamps remapped via TimeMap."
            },
            "subtitles": {
                "type":"string",
                "enum":["ignore","prefer","require"],
                "description":"Subtitle policy for audio.url (default ignore)."
            },
            "provider": {
                "type":"string",
                "description":"Optional URL resolver hint (auto|youtube|direct|…)."
            },
            "overwrite": {
                "type":"boolean",
                "default": true,
                "description":"Overwrite existing outputs next to the source (default true)."
            },
            "docs": {
                "type":"string",
                "description":"Path to accompanying documents/materials (folder or file). Fed to prepare-context → vd-assets for fix-asr / fix-terms (glossary, names, domain terms)."
            },
            "output_dir": {"type":"string"}
        }
    })
}

fn meeting_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "audio": input_source_schema(),
            "inputs": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["role"],
                    "properties": {
                        "role": {
                            "type":"string",
                            "enum":["room","merged","participant","context"],
                            "description":"room/merged = shared mix; participant = per-speaker track; context = docs/materials for vd-assets"
                        },
                        "path": {"type":"string"},
                        "uri": {"type":"string"},
                        "url": {"type":"string"},
                        "artifact": {"type":"string"},
                        "blob": {"type":"string"},
                        "subtitles": {"type":"string","enum":["ignore","prefer","require"]},
                        "participant": {"type":"string","description":"Speaker id/name when role is participant"},
                        "purposes": {"type":"array","items":{"type":"string"}}
                    }
                }
            },
            "meeting": {
                "type":"object",
                "description":"Meeting model: participants.known (name, constraints.gender), diarization.enabled (auto|true|false)"
            },
            "output": {"type":"object"},
            "working_dir": {"type":"string"},
            "document": {"type":"string"},
            "meeting_yaml": {"type":"string"},
            "options": {"type":"object"},
            "execute": {"type":"boolean","default":true},
            "run": {"type":"boolean","description":"Alias for execute"},
            "engine": {"type":"string"},
            "model": {"type":"string"},
            "device": {
                "type":"string",
                "description":"ASR device (cpu|metal|auto). On macOS defaults to metal when omitted. Metal OOM auto-retries on CPU inside vd-gigaam."
            }
        }
    })
}
