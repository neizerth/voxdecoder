# vd-srv — project layout

**Execution engine**: schedule **nodes** · persist · observe. Capability work stays in the shared Executor (`vd-pipeline`).

**Status: implemented (v1).** Path: `src/cli/manage/vd-srv`.

Related: [README.md](README.md) · [cli.md](cli.md) · [TRANSPORT.md](TRANSPORT.md) · [RUST.md](RUST.md) · [`vd-pipeline`](../../process/vd-pipeline/) · [`vd-meeting`](../../process/vd-meeting/)

**v1 note:** workers dispatch **whole Jobs** to the shared Executor; node FSM + Event Store power queue/watch; control plane is JSON-RPC 2.0 over UDS (optional TCP). Fine-grained per-node Worker dispatch and Windows Named Pipe are next — see [TRANSPORT.md](TRANSPORT.md).

---

## Philosophy

```text
Frontends  →  Job Store  →  Node Scheduler  →  Worker Pool  →  Executor  →  Capabilities
```

- **`vd-srv` is the execution engine** — queue is one subsystem, not the whole product.
- **Job** = unit of **submission** (file or stdin; same schema as `vd-pipeline` / planned by `vd-meeting`).
- **Node** = unit of **scheduling**, Resource Class leases, and dispatch.
- Meeting semantics (`role: room`, `purposes: timeline|transcript`) live in **Job builders**; `vd-srv` only schedules the emitted DAG.
- **Job Store** = source of truth; **Event Store** = immutable append-only history.
- Scheduler is **capability-agnostic** (deps · priority · resources · workers · retry).
- Foreground CLIs may call the Executor directly; `vd-srv` is durable background execution.

See [README § Scheduling Model](README.md#scheduling-model).

---

## Non-goals

- Second Executor / forked Job schema
- Scheduler that understands domain capability names
- “Executor Pool” (Executor is singular; pool is Workers)
- Requiring hand-edited `events.ndjson` for operators (`events` / `watch` CLIs exist)
- Cloud-required control plane
- Embedding ASR / diarize / LLM backends here
- Silent loss of Jobs on restart
- Non-idempotent capabilities without documenting resume risk
- Platform-specific daemons as hard requirements

---

## Tree (target)

```
src/cli/manage/vd-srv/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── TRANSPORT.md                # JSON-RPC + IPC / TCP transport contract
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── paths.rs
│   ├── cli/                    # serve · submit · watch · logs · events · artifacts · top · …
│   ├── config/                 # workers · resource_classes · http · socket · retention · …
│   ├── api/                    # JSON-RPC + transport (uds · tcp · pipe stub)
│   ├── store/
│   │   ├── job.rs              # Job Store (source of truth)
│   │   ├── event.rs            # Event Store (immutable, append-only)
│   │   └── artifact.rs         # Artifact Store + artifacts.json
│   ├── schedule/
│   │   ├── mod.rs              # Node Scheduler
│   │   └── resources.rs        # Resource Class entities + leases
│   ├── pool/                   # Worker Pool
│   ├── observe/                # health · top · metrics · timeline · explain · watch
│   ├── retention/              # TTL for artifacts / logs / events
│   └── recovery/               # restart policy: never | resume | retry
│
└── tests/
    ├── unit/
    ├── integration/
    ├── e2e/
    └── fixtures/
        ├── jobs/
        └── store/
```

---

## Module map

| Area | Owns |
|------|------|
| `cli/` | All operator commands including `watch` · `logs` · `events` · `artifacts` · `worker info` |
| `api/` | Submit (body/stdin equivalent) · cancel · streams |
| `store/job` | Job + DAG + priority + restart policy + per-node state |
| `store/event` | Append-only events for `watch` / `events --follow` |
| `store/artifact` | `artifacts.json` + listing CLI |
| `schedule/` | Node readiness + selection (deps · priority · resources · workers) |
| `schedule/resources` | Resource Class **entities** (capacity, future metadata) |
| `pool/` | Workers → Executor per node |
| `observe/` | `top`, metrics, explain, …
| `retention/` | Apply TTLs |
| `recovery/` | `never` / `resume` / `retry` |

---

## Data layout (default)

```text
$VD_SRV_DATA/
├── config.toml
├── server.pid
├── socket
├── jobs/
│   └── <job-id>/
│       ├── job.yaml
│       ├── resolved.json
│       ├── state.json
│       ├── artifacts.json
│       ├── events.ndjson
│       ├── stdout.log
│       ├── stderr.log
│       ├── metrics.json
│       └── timeline.json
├── artifacts/
└── metrics/
    └── latest.json
```

---

## State machines

### Job

```text
Submitted → Queued → Running → Completed | Failed | Cancelled
```

### Node

```text
Pending
 → WaitingDependencies
 → WaitingResources
 → Ready
 → Running
 → Completed | Failed | Cancelled | Skipped
```

### Queue buckets

`Queued` · `WaitingDependencies` · `WaitingResources` · `Ready` · `Running` · terminal.

---

## Scheduling vs Executor

| Concern | Owner |
|---------|--------|
| Deps / priority / Resource Classes / Worker free | **Node Scheduler** |
| Run one resolved node | **Executor** (via Worker) |
| Restart policy | recovery + Job document |
| Retention TTLs | retention module |
| Live follow | Event Store → `watch` / `events --follow` |

---

## Resource Classes

Entities in config (open-ended):

```yaml
resource_classes:
  cuda_gpu: { capacity: 2 }
  metal_gpu: { capacity: 1 }
  cpu: { capacity: 12 }
  ram: { capacity: 64GB }
```

Nodes request classes by name; Manager leases capacity for the node duration.

---

## Config keys (contract)

`workers` · `resource_classes` · `http` · `socket` · `retention` · `history` · `log_level`

---

## Restart policy

Per Job: `never` | `resume` | `retry`. Prefer idempotent capabilities.

---

## Retention

```yaml
retention:
  artifacts: 30d
  logs: 14d
  events: forever
```

---

## Metrics (contract)

Keep separate: `queue_wait_ms` · `resource_wait_ms` · `dependency_wait_ms` · `step_wait_ms` · `step_run_ms` · `job_total_ms`.

---

## Tests (planned)

| Layer | What |
|-------|------|
| unit | Store · Resource Classes · node FSM · retention · restart policy |
| integration | stdin submit · watch stream · stub Executor per node |
| e2e | serve + pipe from `vd-pipeline` / `vd-meeting plan --json` + watch/logs |

Gates: [RUST.md](RUST.md).

---

## Implementation order (suggested)

1. Job + Event stores + node FSM  
2. `serve` / `submit` (file + stdin) / `job info` / `queue`  
3. Scheduler (deps) + one Worker  
4. Resource Classes + WaitingResources / Ready  
5. Multi-worker + priority  
6. `watch` / `events` / `logs` / `artifacts`  
7. Restart policy + recovery  
8. Retention + `top` / metrics / `worker info` / `doctor`

Out of scope for v1: distributed workers, PostgreSQL, Prometheus scrape, web dashboard.
