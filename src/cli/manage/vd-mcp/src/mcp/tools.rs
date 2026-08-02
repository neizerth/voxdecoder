//! MCP tool registry and Runtime API forwarding.

use serde_json::{json, Value};

use crate::client::RuntimeClient;

pub fn list() -> Vec<Value> {
    vec![
        tool(
            "process_audio",
            "Plan and optionally execute an audio processing Job.",
            audio_schema(),
        ),
        tool(
            "process_meeting",
            "Plan and optionally execute a meeting Job.",
            meeting_schema(),
        ),
        tool(
            "submit_job",
            "Submit a complete Runtime Job document.",
            json!({"type":"object","properties":{"job":{},"job_yaml":{"type":"string"},"document":{"type":"string"}}}),
        ),
        tool("get_job", "Get a Job record by id.", id_schema()),
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
        "process_audio" => client.call("plan.audio", Some(arguments)),
        "process_meeting" => client.call("plan.meeting", Some(arguments)),
        "submit_job" => client.call("job.submit", Some(arguments)),
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

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name": name, "description": description, "inputSchema": input_schema})
}

fn id_schema() -> Value {
    json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"}}})
}

fn input_source_schema() -> Value {
    json!({
        "type": "object",
        "description": "Exactly one of path, uri, artifact, or blob.",
        "properties": {
            "path": {"type":"string"},
            "uri": {"type":"string"},
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
            "device": {"type":"string"},
            "docs": {"type":"string"},
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
                        "role": {"type":"string","enum":["room","merged","participant","context"]},
                        "path": {"type":"string"},
                        "uri": {"type":"string"},
                        "artifact": {"type":"string"},
                        "blob": {"type":"string"},
                        "participant": {"type":"string"},
                        "purposes": {"type":"array","items":{"type":"string"}}
                    }
                }
            },
            "meeting": {"type":"object"},
            "output": {"type":"object"},
            "working_dir": {"type":"string"},
            "document": {"type":"string"},
            "meeting_yaml": {"type":"string"},
            "options": {"type":"object"},
            "execute": {"type":"boolean","default":true},
            "run": {"type":"boolean","description":"Alias for execute"},
            "engine": {"type":"string"},
            "model": {"type":"string"}
        }
    })
}
