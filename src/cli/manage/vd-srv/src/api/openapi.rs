//! OpenAPI 3.0 description generated from the HTTP Runtime API surface (ADR 0007).

use serde_json::{json, Value};

/// OpenAPI document for the HTTP transport. Single source for `/openapi.json`.
pub fn document() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "VoxDecoder Runtime API",
            "version": "0.1",
            "description": "HTTP transport for the Runtime API (ADR 0006 / 0007). Semantics match JSON-RPC and gRPC."
        },
        "paths": {
            "/live": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "Liveness probe",
                    "responses": { "200": { "description": "Alive" } }
                }
            },
            "/ready": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "Readiness probe (same payload as /health)",
                    "responses": { "200": { "description": "Ready" } }
                }
            },
            "/health": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "Runtime health (required on every transport)",
                    "responses": { "200": { "description": "Health snapshot" } }
                }
            },
            "/doctor": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "Deep health + server_info",
                    "responses": { "200": { "description": "Doctor report" } }
                }
            },
            "/server_info": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "Runtime discovery including transports",
                    "responses": { "200": { "description": "Server info" } }
                }
            },
            "/openapi.json": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "OpenAPI 3.0 JSON",
                    "responses": { "200": { "description": "OpenAPI document" } }
                }
            },
            "/openapi.yaml": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "OpenAPI 3.0 YAML",
                    "responses": { "200": { "description": "OpenAPI document" } }
                }
            },
            "/docs": {
                "get": {
                    "tags": ["Operator"],
                    "summary": "Minimal API docs landing page",
                    "responses": { "200": { "description": "HTML" } }
                }
            },
            "/planning/audio": {
                "post": {
                    "tags": ["Planning"],
                    "summary": "Plan / submit audio Job",
                    "responses": { "200": { "description": "Job or plan" } }
                }
            },
            "/planning/meeting": {
                "post": {
                    "tags": ["Planning"],
                    "summary": "Plan / submit meeting Job",
                    "responses": { "200": { "description": "Job or plan" } }
                }
            },
            "/jobs": {
                "get": {
                    "tags": ["Execution"],
                    "summary": "List Jobs",
                    "responses": { "200": { "description": "Job list" } }
                },
                "post": {
                    "tags": ["Execution"],
                    "summary": "Submit Job",
                    "responses": { "200": { "description": "Submitted Job" } }
                }
            },
            "/jobs/{id}": {
                "get": {
                    "tags": ["Execution"],
                    "summary": "Job status",
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": { "200": { "description": "Job record" } }
                }
            },
            "/jobs/{id}/cancel": {
                "post": {
                    "tags": ["Execution"],
                    "summary": "Cancel Job",
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": { "200": { "description": "Cancelled Job" } }
                }
            },
            "/jobs/{id}/events": {
                "get": {
                    "tags": ["Events"],
                    "summary": "Job event stream (SSE)",
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": { "200": { "description": "text/event-stream" } }
                }
            }
        }
    })
}

pub fn yaml() -> Result<String, String> {
    serde_yaml::to_string(&document()).map_err(|e| e.to_string())
}

pub fn docs_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>VoxDecoder Runtime API</title></head>
<body>
<h1>VoxDecoder Runtime API</h1>
<p>HTTP transport (ADR 0006 / 0007). Machine-readable OpenAPI:</p>
<ul>
<li><a href="/openapi.json">/openapi.json</a></li>
<li><a href="/openapi.yaml">/openapi.yaml</a></li>
<li><a href="/health">/health</a></li>
</ul>
</body>
</html>
"#
    .into()
}
