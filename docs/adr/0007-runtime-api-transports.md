# ADR 0007 — Runtime API Transports

**Status:** Accepted  
**Type:** ADR  
**Date:** 2026-08-02

**Related:** [`vd-srv`](../../src/cli/manage/vd-srv/) · [`vd-mcp`](../../src/cli/manage/vd-mcp/) · [`vdctl`](../../src/cli/manage/vdctl/) · [ADR 0006 — HTTP Transport](0006-http-transport-for-runtime-api.md) · [`TRANSPORT.md`](../../src/cli/manage/vd-srv/TRANSPORT.md)

---

## Motivation

The Runtime API is the single public contract of the VoxDecoder Runtime.

All clients — CLI, Desktop, MCP, Web UI, automation, and third-party integrations — must communicate through the same Runtime API.

Different clients require different transport technologies, but they must expose identical Runtime semantics.

The Runtime should not invent proprietary protocols where mature standards already exist.

---

## Core rule

```text
The Runtime API is transport-independent.

HTTP and gRPC are transport implementations.

Behavior must remain identical regardless
of the selected transport.
```

Transport never changes Runtime semantics.

---

## Runtime API

The Runtime API consists of four logical services.

```text
Planning API

Execution API

Operator API

Event API
```

Every transport exposes the same APIs.

---

## Architecture

```text
                    Runtime API

        Planning
        Execution
        Operator
        Events

               │

      ┌────────┴────────┐

      ▼                 ▼

 HTTP / JSON          gRPC

      │                 │

   TCP / UDS        TCP / UDS

               │

               ▼

             vd-srv

               │

        Executor
```

MCP is **not** a Runtime transport.

It is an external protocol translated by `vd-mcp` into Runtime API calls.

### Relation to native IPC

Native local control plane remains JSON-RPC 2.0 over UDS / Named Pipe / TCP — see [`TRANSPORT.md`](../../src/cli/manage/vd-srv/TRANSPORT.md). That plane is the default for `vdctl`, `vd-mcp`, and Desktop.

This ADR defines the **optional standard transports** (HTTP, gRPC) that expose the same Planning / Execution / Operator / Event APIs. ADR 0006 details HTTP; gRPC is specified here and implemented later.

---

## Supported transports

### HTTP

Optional.

Intended for:

- browsers
- curl
- scripting
- automation
- AI agents
- Desktop diagnostics
- Kubernetes probes

Uses JSON payloads.

Disabled by default.

See [ADR 0006](0006-http-transport-for-runtime-api.md).

### gRPC

Optional.

Intended for:

- Desktop
- mobile applications
- high-performance clients
- typed SDKs
- streaming

Uses protobuf.

Disabled by default.

### MCP

Handled exclusively by `vd-mcp`.

`vd-mcp` is a Runtime API client.

It never talks to Executors directly.

---

## HTTP API

The HTTP transport exposes REST endpoints.

### Planning API

```text
POST /planning/audio

POST /planning/meeting
```

### Execution API

```text
POST /jobs

GET /jobs

GET /jobs/{id}

POST /jobs/{id}/cancel
```
### Operator API

```text
GET /health

GET /ready

GET /live

GET /doctor

GET /server_info
```

---

## Event streaming

HTTP supports long-running operations through **Server-Sent Events (SSE).**

```text
GET /jobs/{id}/events
```

Example stream:

```text
JobQueued

JobStarted

NodeStarted

NodeProgress

ArtifactProduced

NodeCompleted

JobCompleted
```

SSE is the canonical HTTP event transport.

Polling should not be required.

---

## gRPC services

| Logical API | Service |
|-------------|---------|
| Planning | `PlanningService` |
| Execution | `ExecutionService` |
| Operator | `OperatorService` |
| Events | `EventService` |

Streaming uses native gRPC streams.

---

## OpenAPI

HTTP automatically exposes an OpenAPI description.

```text
GET /openapi.json

GET /openapi.yaml
```

Optional documentation UI:

```text
/docs
```

or

```text
/swagger
```

Implementation is configurable.

OpenAPI is generated from the Runtime API implementation.

It must never become a separately maintained document.

---

## Configuration

Runtime configuration:

```toml
http.enabled = true
http.bind = "127.0.0.1:7701"

grpc.enabled = true
grpc.bind = "127.0.0.1:7702"
```

Both transports are optional.

Neither is enabled automatically.

---

## Discovery

`server_info` reports transport availability.

Example:

```yaml
runtime:
  version: 1.0
  api_version: 1

transports:
  http:
    enabled: true
    endpoint: http://127.0.0.1:7701

  grpc:
    enabled: true
    endpoint: grpc://127.0.0.1:7702
```

Clients should discover transports instead of assuming them.

---

## Kubernetes

HTTP enables standard probes.

```text
GET /live

GET /ready
```

No custom probe protocol is required.

---

## AI integrations

HTTP enables:

- curl
- browser inspection
- local scripting
- AI coding assistants

without requiring custom socket clients.

SSE enables AI assistants to observe Job progress in real time.

---

## Desktop

Desktop may choose either transport.

Recommended:

```text
Desktop
  ↓
gRPC
```

HTTP remains useful for debugging and diagnostics.

---

## Runtime API compatibility

Every Runtime transport must expose identical semantics.

```text
Planning

Execution

Operator

Events
```

Changing transport must never require client logic changes.

---

## Non-goals

The Runtime will not implement:

- proprietary TCP protocols
- duplicated Runtime implementations
- transport-specific Runtime behavior
- separate HTTP and gRPC feature sets

---

## Success criteria

- Runtime API remains transport-independent.
- HTTP and gRPC are optional Runtime transports.
- HTTP exposes REST, OpenAPI, and SSE.
- gRPC exposes typed services and streaming.
- **Every Runtime transport exposes health** (`server.health` / `GET /health` / `OperatorService.Health`) with identical Engine semantics.
- MCP continues to operate exclusively through `vd-mcp`.
- All clients observe identical Runtime behavior regardless of transport.
