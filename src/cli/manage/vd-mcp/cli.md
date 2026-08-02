# vd-mcp CLI / MCP surface

**MCP Gateway** for the VoxDecoder **Runtime API**. Forwards MCP tools → Runtime requests. Never plans or executes Jobs.

**Status: planned.**

Product: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md). Rust gates: [RUST.md](RUST.md).  
Runtime: [`vd-srv`](../vd-srv/) · Transport: [`TRANSPORT.md`](../vd-srv/TRANSPORT.md) · Platform: [`docs/runtime.md`](../../../../docs/runtime.md).  
Shared inputs: [`docs/input-source.md`](../../../../docs/input-source.md).

---

## Equal clients

```text
Every frontend in VoxDecoder
(MCP, Desktop, CLI, REST, Web)

communicates with the Runtime
through exactly the same Runtime API.

No frontend has privileged access
to internal Runtime components.
```

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

---

## Architecture

```text
MCP host
   │  MCP
   ▼
vd-mcp                    (Runtime Client only)
   │  Runtime API
   ▼
vd-srv
   Planning → Execution → Operator
   Planner → Job → Scheduler → Executor → Capabilities
```

Planners live in the Runtime. The gateway does not embed them.

---

## Runtime API

The Runtime API is the **public contract** between clients and the Runtime.

`vd-mcp` is one of its clients.

```text
Planning API     Domain Request → (plan) → Job [→ execute]
Execution API    Jobs: submit · cancel · get · list · subscribe · pause/resume/retry/clone (future)
Operator API     health · doctor · server_info / discovery
```

Wire methods (illustrative): `submit` · `cancel` · `subscribe` · `artifacts` · `health` · `doctor` · `info`.

---

## Overview

### Process modes

| Mode | Description |
|------|-------------|
| `vd-mcp serve` | MCP server for IDE / Claude Desktop |
| `vd-mcp info` | Connected Runtime, transport, versions, latency |
| `vd-mcp ping` | Runtime reachability (no Job) |
| `vd-mcp doctor` | **Gateway Doctor** (link / protocol / compatibility) |
| `vd-mcp config` | Gateway defaults |

Bare `vd-mcp` may insert `serve` when launched as an MCP server.

---

## Tool groups

### Planning API — Domain Requests

| Tool | Description |
|------|-------------|
| `process_audio` | Accept an **AudioRequest**. The Runtime resolves it into a Job. |
| `process_meeting` | Accept a **MeetingRequest**. The Runtime resolves it into a Job. |

```yaml
execute: true    # default — plan + execute
execute: false   # plan only — return Job, do not run
# alias: run: false
```

### Execution API — Jobs & control

| Tool | Description |
|------|-------------|
| `submit_job` | Submit a full Job |
| `get_job` | State, node, progress, timings, outputs |
| `cancel_job` | Cancel by JobId |
| `list_jobs` | Recent Jobs |
| `list_artifacts` | Artifacts for a Job |

Future Execution API ops (same surface, not MCP-specific): `pause` · `resume` · `retry` · `clone`.

### Operator API

| Tool | Description |
|------|-------------|
| `health` | Runtime health |
| `doctor` | **Runtime Doctor** (GPU, models, runners, CUDA/Metal, …) |
| `server_info` | Discovery for LLMs / Desktop UI |

---

## Serve

```bash
vd-mcp serve
vd-mcp serve --transport tcp --tcp 127.0.0.1:7701
vd-mcp serve --transport uds --socket /tmp/vd-srv.sock
```

| Argument | Default | Description |
|----------|---------|-------------|
| `--transport` | `auto` / env | `auto` · `uds` · `pipe` · `tcp` |
| `--tcp` | `VD_TCP` | Runtime TCP endpoint |
| `--socket` | `VD_SOCKET` | Runtime UDS path |
| `--config` | platform path | Gateway config |

### Env

```text
VD_TRANSPORT=tcp
VD_TCP=runtime:7701

VD_TRANSPORT=uds
VD_SOCKET=/tmp/vd.sock

VD_MCP_CONFIG=/path/to/config.toml
```

---

## Domain Requests vs Jobs

| Category | Examples | Who plans |
|----------|----------|-----------|
| Domain Request | `AudioRequest`, `MeetingRequest`, … | Runtime Planning API |
| Job | full Job document | already planned; Execution API runs it |

---

## InputSource

Shared type — see [`docs/input-source.md`](../../../../docs/input-source.md).

```yaml
audio:
  path: /work/meeting.wav   # or uri | artifact | blob
```

Same `InputSource` for MCP, Desktop, CLI, REST, Web.

---

## Event API (MCP progress)

| Event | Meaning |
|-------|---------|
| `JobQueued` | Accepted |
| `JobStarted` | Execution began |
| `NodeQueued` | Node waiting |
| `NodeStarted` | Node running |
| `NodePhaseChanged` | Phase label (load / read / infer / save / …) |
| `NodeProgress` | Numeric or structured progress |
| `ArtifactProduced` | Artifact registered |
| `ArtifactConsumed` | Downstream node consumed an artifact (DAG trace) |
| `NodeCompleted` | Node OK |
| `JobCompleted` | Success |
| `JobFailed` | Failure |
| `JobCancelled` | Cancelled |

Example trace:

```text
ArtifactProduced (transcript)
  → ArtifactConsumed (meeting-merge)
  → ArtifactProduced (meeting)
```

---

## server_info

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

MCP tool `doctor` (or Runtime API equivalent):

```text
checks GPU
checks models
checks runners
checks CUDA / Metal
checks resource classes
```

Do not conflate the two.

---

## Errors

```text
Runtime API error → MCP tool error
```

---

## Config

```bash
vd-mcp config get transport
vd-mcp config set transport tcp
vd-mcp config path
```

Priority: CLI > env > config file > default.

---

## Docker / k8s

Image: `voxdecoder/mcp` — optional; no GPU. See [`docs/runtime.md`](../../../../docs/runtime.md).

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
