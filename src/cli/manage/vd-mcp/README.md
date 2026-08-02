# vd-mcp — MCP Gateway

**Status:** implemented (v0).

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI / MCP surface: [cli.md](cli.md).  
Rust gates: [RUST.md](RUST.md).  
Sibling Runtime: [`vd-srv`](../vd-srv/) · Transport: [`TRANSPORT.md`](../vd-srv/TRANSPORT.md).  
Platform: [`docs/runtime.md`](../../../../docs/runtime.md) · [ADR 0002](../../../../docs/adr/0002-build-and-container-strategy.md).  
Shared inputs: [`docs/input-source.md`](../../../../docs/input-source.md).

## Core rule

```text
vd-mcp is an MCP Gateway for the VoxDecoder Runtime.

It translates MCP tool invocations into Runtime API requests.

It never executes Jobs,
plans Jobs,
or accesses capabilities directly.

Its only dependency is the Runtime API exposed by vd-srv.
```

```text
Every frontend in VoxDecoder
(MCP, Desktop, CLI, REST, Web)

communicates with the Runtime
through exactly the same Runtime API.

No frontend has privileged access
to internal Runtime components.
```

Planner implementations live **in the Runtime**. `vd-mcp` only forwards **Requests** (and Execution / Operator calls).

---

## Runtime API Stability

```text
The Runtime API is the public contract.

Gateways, Desktop applications,
CLI frontends,
and future services
must depend only on this API.

Capability implementations,
Executors,
and internal planners
may evolve without affecting clients.
```

---

## Runtime API

The Runtime API is the public contract between clients and the Runtime.

`vd-mcp` is one of its clients.

```text
Claude · Cursor · VS Code · Desktop · CLI · REST
                    │
                    ▼
              Runtime API
                    │
                    ▼
                  vd-srv
         Planning API · Execution API · Operator API
                    │
                    ▼
                Executor
```

| API group | Role |
|-----------|------|
| **Planning API** | Accept Domain Requests (`AudioRequest`, `MeetingRequest`, …); Runtime plans Jobs |
| **Execution API** | Run and control Jobs (`submit`, `cancel`, `get`, `list`, `subscribe`; later `pause` / `resume` / `retry` / `clone`) |
| **Operator API** | `health`, Runtime `doctor`, `server_info` / discovery |

---

## Four-layer architecture

```text
Clients
  Desktop · CLI · MCP · REST · Web
        │
        ▼
Runtime API          ← stable public contract
        │
        ▼
Runtime (vd-srv)
  Planning · Execution · Operator
  Scheduler · Resource Manager · Executor
        │
        ▼
Capabilities
  preprocess · transcribe · diarize · meeting · postprocess · …
```

---

## Architecture (MCP path)

```text
MCP Tool
    ↓
Request                 (domain model)
    ↓
Runtime Client
    ↓
Runtime API
    ↓
Planner                 (inside Runtime)
    ↓
Job
    ↓
Scheduler → Executor → Capabilities
```

There is **no** Planner inside `vd-mcp`.

---

## Responsibilities

`vd-mcp` owns:

* MCP protocol
* tool definitions (thin wrappers over Planning / Execution / Operator API)
* request validation / shaping for MCP
* authentication (if needed)
* Runtime API client (Transport)
* response formatting
* streaming Runtime events as MCP progress
* **Gateway Doctor** (`vd-mcp doctor`)

Everything that plans, schedules, executes, or stores state belongs to `vd-srv`.

---

## Non-goals

`vd-mcp` never:

* plans Jobs
* executes Jobs or capability binaries
* schedules work or manages resources
* owns Job / Event / Artifact stores
* depends on CLI binaries as an API
* gets privileged access to Runtime internals

---

## Runtime API accepts two categories

### Domain Requests

Resolved by Runtime **Planning API** into Jobs:

* `AudioRequest`
* `MeetingRequest`
* future: `VideoRequest`, `PodcastRequest`, …

### Runtime Jobs

Already planned documents:

* `Job`

---

## Tool groups

### Planning API

| Tool | Description |
|------|-------------|
| `process_audio` | Accept an **AudioRequest**. The Runtime resolves it into a Job. |
| `process_meeting` | Accept a **MeetingRequest**. The Runtime resolves it into a Job. |

```yaml
execute: true    # default — plan + execute
execute: false   # plan only — return Job without running
# alias: run: false
```

### Execution API

| Tool | Description |
|------|-------------|
| `submit_job` | Submit a full Job |
| `get_job` | State, current node, progress, timings, outputs |
| `cancel_job` | Cancel execution |
| `list_jobs` | Recent Jobs |
| `list_artifacts` | Artifacts for a Job |

Future: `pause` · `resume` · `retry` · `clone` (Execution API, not MCP-specific).

### Operator API

| Tool | Description |
|------|-------------|
| `health` | Runtime health |
| `doctor` | **Runtime Doctor** |
| `server_info` | Discovery for LLMs / Desktop UI |

---

## InputSource

Shared across all clients — see [`docs/input-source.md`](../../../../docs/input-source.md).

```text
path | uri | artifact | blob
```

---

## Event API (streaming)

```text
JobQueued
JobStarted
NodeQueued
NodeStarted
NodePhaseChanged
NodeProgress
ArtifactProduced
ArtifactConsumed      # DAG trace: consumer read an upstream artifact
NodeCompleted
…
JobCompleted | JobFailed | JobCancelled
```

Example:

```text
ArtifactProduced (transcript)
  → ArtifactConsumed (meeting-merge)
  → ArtifactProduced (meeting)
```

---

## Discovery (`server_info`)

```yaml
runtime:
  version: …
  api_version: …
  transport: …
planners:
  - audio
  - meeting
capabilities: […]
models: […]
runners: […]
resource_classes: […]
```

---

## Doctor: Gateway vs Runtime

### Gateway Doctor

```bash
vd-mcp doctor
```

```text
checks transport
checks protocol / api_version
checks compatibility
checks authentication
checks latency / reachability
```

### Runtime Doctor

Operator tool `doctor` / Runtime API:

```text
checks GPU
checks models
checks runners
checks CUDA / Metal
checks resource classes
```

---

## Transport

| Context | Typical transport |
|---------|-------------------|
| Desktop | Unix Domain Socket |
| Windows | Named Pipe |
| Containers / k8s | TCP |

```text
VD_TRANSPORT=tcp
VD_TCP=runtime:7701

VD_TRANSPORT=uds
VD_SOCKET=/tmp/vd.sock
```

---

## Stateless design

`vd-mcp` is **stateless**. Durable state lives in `vd-srv`. Many gateways may share one Runtime.

---

## Error handling

```text
Runtime API error → MCP tool error
```

---

## Observability

```bash
vd-mcp info
vd-mcp ping
vd-mcp doctor          # Gateway Doctor
```

---

## Deployment

Image: `voxdecoder/mcp` (optional; no GPU): `docker build --target mcp`.

---

## Compatibility

```text
vd-mcp depends only on the Runtime API.

New Runtime capabilities,
new planners,
new transports,
and new Executors
must not require Gateway changes
unless the Runtime API changes.
```

New planners (`video`, `podcast`, …) appear in Runtime + `server_info.planners` without Gateway redesign.

---

## Future compatibility

* UDS ↔ TCP without MCP logic changes
* Desktop / Web / CLI / MCP / REST stay interchangeable clients
* distributed Runtimes need no gateway redesign
