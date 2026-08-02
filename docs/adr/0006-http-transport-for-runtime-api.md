# ADR 0006 — HTTP Transport for the Runtime API

**Status:** Accepted  
**Type:** ADR  
**Date:** 2026-08-02

**Related:** [`vd-srv`](../../src/cli/manage/vd-srv/) · [`vd-mcp`](../../src/cli/manage/vd-mcp/) · [`vdctl`](../../src/cli/manage/vdctl/) · [ADR 0007 — Runtime API Transports](0007-runtime-api-transports.md) · [`TRANSPORT.md`](../../src/cli/manage/vd-srv/TRANSPORT.md)

---

## Motivation

The Runtime API is transport-agnostic.

Today the primary transports are:

- Unix Domain Socket
- Named Pipe
- TCP

This works well for native clients (`vdctl`, `vd-mcp`, Desktop).

However, LLM agents frequently attempt to inspect local services using standard HTTP tools:

```bash
curl http://localhost:7701/health
```

or

```bash
curl --unix-socket ... http://localhost/jobs/<id>
```

HTTP also greatly simplifies:

- debugging
- scripting
- shell automation
- browser inspection
- future Web UI
- Kubernetes readiness/liveness probes

The Runtime should expose HTTP **without creating a second API**.

---

## Core rule

```text
HTTP is another transport of the Runtime API.

It is not another API.

It contains no Runtime logic.

All requests are forwarded to the same
Planning / Execution / Operator APIs.
```

---

## Architecture

```text
                   Runtime API

          ┌──────────┼──────────┐

          ▼          ▼          ▼

      UDS / Pipe     TCP      HTTP

          │          │          │

          └──────────┴──────────┘

                    ▼

                  vd-srv

 Planning API · Execution API · Operator API

                    ▼

                 Executor
```

Transport changes.

Behavior never changes.

---

## Goals

Provide a transport suitable for:

- browsers
- curl
- LLM agents
- Desktop diagnostics
- Kubernetes
- reverse proxies

without duplicating Runtime behavior.

---

## Non-goals

HTTP must never:

- implement scheduling
- own Job state
- bypass Runtime APIs
- expose capability internals
- become a parallel implementation

---

## Transport selection

```text
Native Desktop
    UDS

Windows
    Named Pipe

Container
    TCP

HTTP
    optional
```

HTTP is disabled unless configured.

---

## Configuration

```bash
vd-srv serve \
    --http 127.0.0.1:7701
```

or

```toml
[http]
enabled = true
bind = "127.0.0.1:7701"
```

Default:

```text
disabled
```

---

## Runtime API mapping

### Planning API

```text
POST /planning/audio
POST /planning/meeting
```

### Execution API

```text
POST /jobs
GET /jobs
GET /jobs/:id
POST /jobs/:id/cancel
GET /jobs/:id/events
```

### Operator API

```text
GET /health
GET /doctor
GET /server_info
```

### Future

```text
POST /jobs/:id/retry
POST /jobs/:id/clone
POST /jobs/:id/pause
POST /jobs/:id/resume
```

---

## Event streaming

Support Server-Sent Events (SSE) for Job events.

```text
GET /jobs/:id/events
```

produces the same event model as the Runtime transport (`JobQueued`, `JobStarted`, `NodeStarted`, …).

---

## Health endpoints

For orchestration:

```text
GET /health
GET /ready
```

Used by Kubernetes, Docker, and reverse proxies.

---

## Discovery

```text
GET /server_info
```

Advertises runtime version, available transports, capabilities, models, and runners.

---

## LLM support

HTTP transport allows Skills and agents to use standard tooling:

```bash
curl http://localhost:7701/jobs
```

instead of custom socket clients.

---

## Runtime API compatibility

Every Runtime transport must expose the same behavior.

```text
Runtime API
        ▼
    Transport
        ▼
    Behavior
```

Changing transport must never require changing clients’ Runtime semantics.

---

## Security

By default:

```text
127.0.0.1 only
```

Remote access requires explicit configuration.

Authentication is intentionally out of scope for v1.

---

## Performance

HTTP is a thin adapter:

```text
HTTP → Runtime API → Planning / Execution / Operator
```

No duplicate storage, scheduling, or Runtime. Overhead is negligible compared to audio workloads.

---

## Success criteria

- Runtime API available over HTTP.
- No duplicated Runtime logic.
- Same semantics as UDS / Pipe / TCP.
- SSE provides Runtime events.
- Kubernetes health probes use the same Operator API.
- AI agents can inspect Runtime state using standard HTTP clients.
