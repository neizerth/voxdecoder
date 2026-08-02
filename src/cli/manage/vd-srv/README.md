# vd-srv — Runtime (execution engine)

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI / API surface: [cli.md](cli.md).  
Transport / RPC: [TRANSPORT.md](TRANSPORT.md).  
Platform role: [`docs/runtime.md`](../../../../docs/runtime.md).  
Related: [`vd-pipeline`](../../process/vd-pipeline/) · [`vd-meeting`](../../process/vd-meeting/) · [`vd-postprocess`](../../process/vd-postprocess/) · [`vd-diarize`](../../process/vd-diarize/) · [`vd-mcp`](../vd-mcp/) (planned — MCP Gateway).
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/), [`vd-pipeline`](../../process/vd-pipeline/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented (v1).** Workspace member: `src/cli/manage/vd-srv`.

> **`vd-srv` is the Runtime Environment for VoxDecoder.**
>
> It owns **Planners** (Domain Request → Job), Worker Pool, Resource Classes, Queue, Event/Artifact stores, Health, Transport, and scheduling. Capability logic remains in the shared Executor (lib preferred, CLI subprocess fallback).
>
> **Runtime API** is the public contract. Clients — Desktop, Web UI, CLI (`--via-srv`), REST/gRPC, and [`vd-mcp`](../vd-mcp/) — depend only on this API. Planners and capabilities may evolve behind it. `vd-mcp` forwards Requests; it does not host Planners.

**v1 scope:** Job-granularity workers (full Job → shared `Executor`); node records + Event Store for queue/watch; JSON-RPC 2.0 control plane over transport abstraction (UDS primary; optional TCP) — see [TRANSPORT.md](TRANSPORT.md); filesystem Job Store. Per-node dispatch and Windows Named Pipe are next.

The queue is only one subsystem. Scheduler, Resource Manager, Worker Pool, and Event Store together form the long-running **Runtime** that serves CLI, MCP, HTTP, and a future GUI the same way. `vd-pipeline` and `vd-meeting` plan Jobs; `vd-srv` runs them in durable mode. Container image: `voxdecoder/runtime` ([docs/runtime.md](../../../../docs/runtime.md)).

---

## Core rule

```text
vd-srv is the Runtime for VoxDecoder.

It accepts Jobs, schedules DAG nodes, limits resources, balances concurrent work,
persists state, and exposes progress — all through the shared Executor.

It is not another pipeline implementation.
```

There is **one** Executor in the project (`vd-pipeline`). `vd-srv` owns **node scheduling**.

```text
CLI
MCP
API
        │
        ▼
     vd-srv          ← execution engine
        │
        ▼
     Executor
        │
        ▼
Capabilities
```

| Layer | Role |
|-------|------|
| **Frontends / Runtime API clients** (`vd-pipeline` CLI, `vd-meeting`, `vd-mcp`, HTTP, Desktop, …) | Plan (where needed) · submit · cancel · observe via Runtime API |
| **vd-srv** | Execution engine: persist, schedule **nodes**, Resource Classes, workers, observability |
| **Executor** (`vd-pipeline`) | Run one resolved Job node (capability + options + I/O) |
| **Capabilities** | Domain work (`transcribe`, `fix-*`, `diarize`, …) |

---

## Mission

Responsibilities:

* accept Jobs from CLI / MCP / API (files **or stdin**);
* resolve each Job into a DAG of **nodes**;
* schedule runnable nodes (dependencies + Resource Classes + priority + workers);
* balance concurrent work across a **Worker Pool**;
* execute nodes through the shared Executor;
* persist state (Job Store + Event Store);
* retain history per policy;
* expose progress, logs, events, artifacts, and live observability.

Foreground today: builders call the Executor directly.  
Background: the same Job model goes through `vd-srv`.

---

## Core principles

### Single Executor

Every node is executed by the shared Executor from `vd-pipeline`.

`vd-srv` never contains its own pipeline logic. Its job is **scheduling nodes**.

### Node Scheduler (not Job Scheduler)

The public unit of submission is still a **Job**.

Internally the Scheduler never “runs a Job” as a blob. It works on **Job nodes**:

```text
Job
 ↓
Resolved DAG
 ↓
Ready Nodes
 ↓
Workers
 ↓
Executor
```

* a Job may have **zero, one, or many** runnable nodes at once;
* nodes obtain Resource Class leases;
* nodes are dispatched to idle Workers;
* multi-track meetings are normal DAG parallelism — not a special case for `vd-meeting`.

Example Job from [`vd-meeting`](../../process/vd-meeting/) (room mix = **timeline only**, tracks = **transcript**):

```text
alice.wav ──► transcribe → fix-* ──► alice.text ──┐
bob.wav   ──► transcribe → fix-* ──► bob.text   ──┼──► meeting-merge
meeting.wav ─► diarize ────────────► timeline ────┘
```

Under `vd-srv` those are independent **nodes**. Alice/Bob ASR and room diarize become runnable together when resources allow; `meeting-merge` waits on dependencies. The Scheduler never interprets `role: room` / `purposes` — that was already resolved by the planner into the Job DAG.

### Local-first

All scheduling and orchestration are local. No cloud infrastructure is required.

Optional online providers are used only by Job capabilities such as `postprocess`.

### Persistent state

A server restart must not lose Jobs.

**Job Store is the source of truth for every Job.**

Queue, progress, history, and observability derive from persisted Job state and the Event Store.

### Observable by default

Inspect without a debugger: `queue` · `watch` · `logs` · `events` · `artifacts` · `top` · `workers` · metrics.

---

## Scheduling Model

```text
Scheduler operates on Job Nodes.

A Job may have zero, one, or many runnable nodes simultaneously.

Runnable nodes are selected according to:
  • dependencies
  • priority
  • resource availability
  • worker availability

Scheduler never executes capability code.
```

Nodes become runnable when dependencies are complete **and** required Resource Classes can be leased **and** a Worker is free.

The Scheduler is **capability-agnostic**: it does not know `transcribe` / `fix` / `meeting` / `summary` — only resources · dependencies · priority · retry.

Meeting planners (`vd-meeting`) decide which sources need ASR vs diarize (`purposes`). `vd-srv` only sees the resulting nodes and edges. Contended classes (e.g. several `transcribe` + one `diarize` on `metal_gpu`) are handled by the Resource Manager, not by meeting-specific rules.

This section defines server behavior. Parallelism, waiting, and balancing are properties of the model.

---

## High-level architecture

```text
               CLI
                │
        MCP     │
         │      │
         ▼      ▼
            API Layer
                │
────────────────────────────────────────
            Job Store          ← source of truth
            Event Store        ← immutable, append-only
            Artifact Store
            Scheduler          ← nodes, not Jobs
            Resource Manager   ← Resource Class entities
            Worker Pool
────────────────────────────────────────
             Executor   (vd-pipeline)   ← one shared runtime
────────────────────────────────────────
transcribe · prepare-context · diarize
meeting-merge · fix-* · postprocess · …
```

```text
Scheduler
    │
Worker Pool
    │
Executor
```

---

## Components

### API Layer

Receives Jobs from `vd-pipeline`, `vd-meeting`, `vd-mcp`, GUI, HTTP, local socket — including **stdin / piped JSON**.

Responsibilities: submit · cancel · status · stream events · logs · artifacts.

### Job Store

**Source of truth for every Job.**

Stores: Job document · resolved DAG · priority · restart policy · state · timestamps · retries · exit codes · per-node state.

Backends: filesystem (default) · SQLite · PostgreSQL (future).

### Event Store

**Events are immutable and append-only.**

Enables replay, audit, `watch` / `events --follow`, and rebuilding projections.

```text
JobQueued · JobStarted · JobWaitingResources
NodePending · NodeWaitingDependencies · NodeWaitingResources
NodeReady · NodeStarted · NodeProgress · NodeFinished
ArtifactProduced
JobFinished · JobFailed · JobCancelled
```

### Artifact Store

Tracks produced artifacts: id · path · type · producer · timestamps.

Per Job: `artifacts.json`. CLI: `vd-srv artifacts <id>`.

Future: cache · deduplication · retention (see below).

### Scheduler

**Capability-agnostic node scheduler.**

Selection inputs: dependencies · priority · Resource Class availability · Worker availability.

Never executes capability code. Dispatches **ready nodes** to the Worker Pool.

### Resource Manager

Manages **Resource Class entities** (not free-form strings alone).

Server config example:

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

Later classes (`license`, `network`, `huggingface`, `llm`, …) extend the same model without changing the Scheduler.

Nodes declare requirements; while waiting they sit in `WaitingResources`.

### Worker Pool

Workers (processes or threads — TBD). Each runs **one node at a time** through the shared Executor.

```text
Worker 1 — node:transcribe/alice
Worker 2 — node:postprocess/summary
Worker 3 — idle
```

Inspect: `vd-srv workers` · `vd-srv worker info <n>`.

---

## Executor contract

| Direction | Payload |
|-----------|---------|
| Scheduler → Worker → Executor | **Resolved Job Node** |
| Executor → events / Worker | **Started** · **Progress** · **Artifacts** · **Completed** · **Failed** |

Under `vd-srv`, readiness and leasing live in the server; Workers invoke the Executor **per node**.

---

## Job lifecycle

```text
Submitted
    ↓
Queued
    ↓
Running          ← one or more nodes active / waiting
    ↓
Completed | Failed | Cancelled
```

A Job is `Running` while any node is not terminal. Fine-grained waiting lives on **nodes** (and surfaces in `queue` / `explain`).

### Node status

```text
Pending
WaitingDependencies
WaitingResources
Ready
Running
Completed
Failed
Cancelled
Skipped
```

These states power `explain`, `watch`, and queue breakdowns.

### Queue views

Do **not** collapse everything into a single “pending” bucket. Distinct buckets:

| Bucket | Meaning |
|--------|---------|
| `Queued` | Accepted; not yet considered for dispatch |
| `WaitingDependencies` | Upstream nodes incomplete |
| `WaitingResources` | Deps ok; Resource Classes unavailable |
| `Ready` | Runnable; waiting for a free Worker |
| `Running` | On a Worker |
| `Completed` / `Failed` / `Cancelled` | Terminal (recent window) |

---

## Priority

Jobs carry priority (document field and/or submit flag):

```yaml
priority: low   # low | normal | high | … (exact enum TBD)
```

```bash
vd-srv submit job.yaml --priority high
```

Scheduler prefers higher priority among otherwise equal ready nodes.

---

## Restart policy

Per Job:

```yaml
restart: resume   # never | resume | retry
```

| Policy | On server restart / worker crash |
|--------|----------------------------------|
| `never` | Leave unfinished nodes failed / cancelled |
| `resume` | Continue unfinished nodes (preferred default) |
| `retry` | Re-run failed/incomplete nodes from scratch |

**Capabilities should be idempotent whenever practical** — required for safe `resume` / `retry`.

---

## Retention

```yaml
retention:
  artifacts: 30d
  logs: 14d
  events: forever
```

Server-wide defaults; optional per-Job overrides later.

---

## Progress and timings

| Scope | Fields |
|-------|--------|
| Job | `queued_at` · `started_at` · `finished_at` · `job_total_ms` · `queue_wait_ms` |
| Node | `step_run_ms` · `step_wait_ms` · **`resource_wait_ms`** · **`dependency_wait_ms`** |

`queue_wait_ms`, `resource_wait_ms`, and `dependency_wait_ms` answer different questions — keep them separate.

Live UI: `vd-progress` (no timings). Durable numbers: Job / Event Store.

---

## Observability

| Command | Purpose |
|---------|---------|
| `vd-srv ping` / `health` / `doctor` | Reachability · deep health · env |
| `vd-srv top` | Live CPU · GPU · queue · workers · Jobs · memory |
| `vd-srv queue` / `jobs` / `job info` | Inventory |
| `vd-srv watch <id>` | Follow Job via Event Store |
| `vd-srv events <id> [--follow]` | Raw / pretty events (no hand-reading `events.ndjson`) |
| `vd-srv logs <id> [--follow]` | Job logs (docker-style) |
| `vd-srv artifacts <id>` | Artifact paths listing |
| `vd-srv timeline` / `trace` / `explain` | Timing bars · DAG · wait reason |
| `vd-srv workers` / `worker info <n>` | Pool + single worker detail |
| `vd-srv metrics` / `profile` | Counters + latency + time share |

HTTP: `/live` · `/ready` · `/jobs` · `/jobs/:id/events` · `/metrics` (future). Details: [cli.md](cli.md).

---

## Logging

```text
jobs/<job-id>/
  artifacts.json
  events.ndjson
  stdout.log
  stderr.log
  metrics.json
  timeline.json
```

Operators use `logs` / `events` / `artifacts` CLIs — not raw files (files remain the durable backend).

---

## Config surface

First-class keys (avoid ad-hoc growth later):

```text
workers
resource_classes
http
socket
retention
history
log_level
```

Plus data-dir / paths. Priority: CLI > env (`VD_SRV_*`) > config file > defaults.

---

## Platform support

Identical behavior on Windows · Linux · macOS — no platform-specific services required.

---

## Future extensions

Without changing the Job contract:

* distributed workers · remote Executors · more Resource Classes
* scheduled Jobs · web dashboard · Prometheus / OpenTelemetry
* distributed artifact storage · event replay tools

---

## Public contract

`vd-srv` is the **execution engine** for VoxDecoder.

It owns node scheduling, persistence, Resource Class management, Worker coordination, retention, and observability.

Pipeline logic remains in the shared Executor.

Every frontend submits the **same** Job model (file or stdin) and observes execution through the **same** event stream.
