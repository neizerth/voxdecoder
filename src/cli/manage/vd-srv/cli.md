# vd-srv CLI

**Execution engine** CLI: submit Jobs, schedule **DAG nodes**, persist state, observe progress. Capability work uses the shared Executor from [`vd-pipeline`](../../process/vd-pipeline/).

**Status: implemented (v1).**

Product notes: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md). Transport: [TRANSPORT.md](TRANSPORT.md).

v1: JSON-RPC 2.0 control plane over UDS (optional TCP); Job-granularity workers; `submit` accepts file or `-` (stdin YAML/JSON). See [TRANSPORT.md](TRANSPORT.md).

---

## Architecture

```text
vd-pipeline / vd-meeting / MCP  ──pipe──▶  vd-srv submit -
                                              ↓
                                    Job Store + Event Store
                                              ↓
                                    Node Scheduler + Resource Classes
                                              ↓
                                    Worker Pool → Executor [per node]
```

`vd-pipeline run` remains the foreground one-shot path.  
`vd-srv` is the durable execution engine for the **same** Job document.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-srv serve` | Start the server |
| `vd-srv stop` | Stop the server |
| `vd-srv ping` | Reachability |
| `vd-srv health` | Deep health |
| `vd-srv top` | Live dashboard |
| `vd-srv doctor` | Runtime environment |
| `vd-srv submit` | Enqueue Job (**file or `-` stdin**) |
| `vd-srv cancel <id>` | Cancel a Job |
| `vd-srv queue` | Queue by node buckets |
| `vd-srv jobs` | List Jobs |
| `vd-srv job info <id>` | Job + node detail |
| `vd-srv watch <id>` | Follow Job via Event Store |
| `vd-srv events <id>` | Print events (`--follow`) |
| `vd-srv logs <id>` | Print logs (`--follow`) |
| `vd-srv artifacts <id>` | List artifact paths |
| `vd-srv timeline <id>` | Per-node timings |
| `vd-srv trace <id>` | DAG execution view |
| `vd-srv explain <id>` | Why waiting (deps vs resources) |
| `vd-srv workers` | Worker table |
| `vd-srv worker info <n>` | Single worker detail |
| `vd-srv metrics` | Counters / latencies |
| `vd-srv profile` | Aggregated time share |
| `vd-srv config` | Server defaults |

HTTP transport (ADR 0006 / 0007, **disabled by default**): `--http 127.0.0.1:7701` or `[http] enabled = true`. Routes forward to the same Runtime APIs:

- Planning: `POST /planning/audio` · `POST /planning/meeting`
- Execution: `POST /jobs` · `GET /jobs` · `GET /jobs/:id` · `POST /jobs/:id/cancel` · `GET /jobs/:id/events` (live SSE)
- Operator: `GET /health` · `GET /ready` · `GET /doctor` · `GET /server_info` · `GET /live` · `GET /openapi.json` · `GET /docs`

gRPC transport (ADR 0007, **disabled by default**): `--grpc 127.0.0.1:7702` or `[grpc] enabled = true`. Services: `PlanningService`, `ExecutionService`, `OperatorService` (**Health** required), `EventService.WatchJob`. Observe is typed (`JobView` / `Event` / `HealthResponse`); Planning + Submit stay `JsonBody`. See [proto/README.md](proto/README.md).

Health is available on **every** transport (`server.health` / `GET /health` / `OperatorService.Health`).

---

## Serve

```bash
vd-srv serve --http 127.0.0.1:7701
vd-srv serve --grpc 127.0.0.1:7702
vd-srv serve --tcp 127.0.0.1:7700 --http 127.0.0.1:7701 --grpc 127.0.0.1:7702
```

| Argument | Default | Description |
|----------|---------|-------------|
| `--data-dir` | platform data dir | Durable root |
| `--socket` | under data-dir | Unix domain socket path (IPC) |
| `--tcp` | off | TCP bind; also adds a secondary listener when primary is IPC |
| `--http` | off | HTTP transport bind (ADR 0006); REST/SSE/OpenAPI → Runtime API |
| `--grpc` | off | gRPC transport bind (ADR 0007); includes OperatorService.Health |
| `--transport` | `auto` | `auto` · `uds` · `pipe` · `tcp` — see [TRANSPORT.md](TRANSPORT.md) |
| `--workers` | config / `1` | Worker Pool size |
| `--foreground` | on | Stay attached |

Control plane: JSON-RPC 2.0 (newline-framed) over the selected IPC/TCP transport. Optional HTTP and gRPC listeners (ADR 0006 / 0007) — see above.

Resource Class capacities come from config (`resource_classes`), not only CLI one-offs.

---

## Submit

Accepts a path **or stdin** (`-`):

```bash
vd-srv submit job.yaml
vd-srv submit job.yaml --priority high
vd-srv submit job.yaml --wait --progress=json

vd-pipeline run --dry-run --json | vd-srv submit -
vd-meeting plan --json | vd-srv submit -
cat job.json | vd-srv submit -
```

| Argument | Description |
|----------|-------------|
| `JOB` | Path to YAML/JSON, or `-` for stdin |
| `--priority` | `low` \| `normal` \| `high` (exact enum TBD); overrides document if set |
| `--restart` | `never` \| `resume` \| `retry` (optional override) |
| `--wait` | Block until terminal state |
| `--progress` | With `--wait`: `text` \| `json` on stderr |
| `--json` | Print Job id / final status as JSON |

Exit codes (target): `0` success · `1` failed · `2` usage · `3` server unreachable · `4` cancelled.

---

## Queue / jobs

```bash
vd-srv queue
vd-srv jobs
vd-srv jobs --status running
vd-srv jobs --status waiting-resources
vd-srv job info <id>
vd-srv job info <id> --json
```

`queue` buckets (nodes / Jobs as applicable):

| Bucket | Meaning |
|--------|---------|
| `Queued` | Accepted, not yet considered |
| `WaitingDependencies` | Upstream incomplete |
| `WaitingResources` | Waiting on Resource Classes |
| `Ready` | Runnable, waiting for a Worker |
| `Running` | On a Worker |
| `Completed` / `Failed` / `Cancelled` | Terminal (recent) |

`job info`: nodes · statuses · timings · outputs · errors · paths under `jobs/<id>/`.

---

## Watch

```bash
vd-srv watch <id>
vd-srv watch <id> --json
```

Subscribes to the Event Store (same stream MCP / HTTP will use):

```text
Queued
↓
Running
  ├─ alice/transcribe 61%
  ├─ bob/transcribe   44%
  └─ diarize (room)   80%
↓
meeting-merge
↓
Done
```

Parallel ready nodes show up together in `watch` / `top`; that is expected for multi-track meeting Jobs.

---

## Events

```bash
vd-srv events <id>
vd-srv events <id> --follow
vd-srv events <id> --json
```

Do not require operators to open `events.ndjson` by hand.

---

## Logs

```bash
vd-srv logs <id>
vd-srv logs <id> --follow
vd-srv logs <id> --stderr
```

Docker-style access to `stdout.log` / `stderr.log`.

---

## Artifacts

```bash
vd-srv artifacts <id>
```

```text
meeting.json
meeting.srt
summary.md
tasks.md
```

Human listing from `artifacts.json` — no JSON spelunking required.

---

## Top

```bash
vd-srv top
```

```text
CPU    ████░░░░  42%
GPU    ████████  metal 1/1
MEM    ███░░░░░  6.2 / 16 GB
Queue  queued=1  waiting-deps=0  waiting-resources=1  ready=2  running=3
Workers  busy=2  idle=1
Jobs   abc…  node=transcribe/alice  61%
       def…  WaitingResources  metal_gpu
```

---

## Timeline / trace / explain

```bash
vd-srv timeline <id>
vd-srv trace <id>
vd-srv explain <id>
```

```text
WaitingResources: metal_gpu (0 free / 1 capacity)
```

```text
WaitingDependencies: node "bob.text" incomplete
```

```bash
vd-srv trace <id>
```

```text
alice/transcribe ──┐
bob/transcribe   ──┤
diarize (room)   ──┤
meeting-merge    ──┘
```

(Typical `vd-meeting` plan: track transcripts ∥ room timeline → merge.)

---

## Workers

```bash
vd-srv workers
vd-srv worker info 3
```

```text
Worker   3
Status   Running
Node     transcribe
Job      abc123
Started  12:34:01
```

---

## Metrics / profile

```bash
vd-srv metrics
vd-srv profile
```

Illustrative metrics:

* `jobs_total` · `jobs_running` · `jobs_failed` · `queue_depth`
* `workers_busy` · `workers_idle`
* `cpu_usage` · `memory_usage` · `gpu_usage` · `disk_usage`
* **`queue_wait_ms`** · **`resource_wait_ms`** · **`dependency_wait_ms`**
* **`step_wait_ms`** · **`step_run_ms`** · **`job_total_ms`**
* Resource Class saturation (leased / capacity)

---

## Health / ping / doctor

```bash
vd-srv ping
vd-srv health
vd-srv doctor
```

`doctor`: CUDA · Metal · Python · pyannote · GigaAM · Ollama · API keys · HF token · child binaries.

---

## Config

```bash
vd-srv config list
vd-srv config get workers
vd-srv config set workers 2
vd-srv config path
```

First-class keys:

| Key | Role |
|-----|------|
| `workers` | Worker Pool size |
| `resource_classes` | Entity map: name → capacity (and future metadata) |
| `transport` | `auto` · `uds` · `pipe` · `tcp` |
| `socket` | Unix domain socket path |
| `tcp` | Optional TCP bind (`127.0.0.1:7701`) |
| `pipe` | Windows named pipe path (stub until implemented) |
| `http` | Reserved for HTTP/WebSocket bridge |
| `retention` | `artifacts` / `logs` / `events` TTLs |
| `history` | How many terminal Jobs to keep indexed |
| `log_level` | Server log verbosity |

Example `resource_classes`:

```yaml
resource_classes:
  cuda_gpu:
    capacity: 2
  metal_gpu:
    capacity: 1
  cpu:
    capacity: 12
  ram:
    capacity: 64GB
```

Example `retention`:

```yaml
retention:
  artifacts: 30d
  logs: 14d
  events: forever
```

Priority: CLI flags > env (`VD_SRV_*`) > config file > defaults.

---

## Progress vs timings

| Channel | Content |
|---------|---------|
| `watch` / `NodeProgress` / stderr | Live UI — no durable timings |
| `events` · `metrics.json` · `timeline.json` · `job info` | Persisted timestamps |

---

## Logging layout

```text
jobs/<job-id>/
  artifacts.json
  events.ndjson
  stdout.log
  stderr.log
  metrics.json
  timeline.json
```

---

## Relationship to other CLIs

| Tool | Talks to vd-srv? |
|------|------------------|
| `vd-pipeline` | `run --dry-run --json \| vd-srv submit -`; optional `--via srv` later |
| `vd-meeting` | `plan --json \| vd-srv submit -` (or `run --dry-run --json \| …`); planner already emits parallel transcript nodes + optional room `diarize` |
| `vd-mcp` | submit + `watch` / event stream |
| HTTP / GUI | API Layer |

Same Job schema everywhere. Scheduling granularity is the **node**. Meeting input `purposes` / `role: room` never reach `vd-srv` — only the planned DAG does.
