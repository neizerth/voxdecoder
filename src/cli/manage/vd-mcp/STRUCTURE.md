# vd-mcp — project layout

**MCP Gateway** for the VoxDecoder **Runtime API**. Forwards Requests and Job ops; does **not** host Planners.

**Status: implemented (v0).** Path: `src/cli/manage/vd-mcp`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [`vd-srv`](../vd-srv/) · [`TRANSPORT.md`](../vd-srv/TRANSPORT.md) · [`docs/runtime.md`](../../../../docs/runtime.md)

---

## Philosophy

```text
Clients → Runtime API (stable) → Runtime (Planner · Scheduler · Resources · Executor) → Capabilities
```

| Layer | Knows | Does not know |
|-------|-------|----------------|
| **MCP host** | tools, args, progress | Jobs, planners, capabilities |
| **vd-mcp** | MCP, Request/Job shapes, Runtime Client | how Runtime plans or schedules |
| **Runtime API** | public contract | MCP, Desktop UI |
| **Runtime Planner** | Domain Request → Job | MCP |
| **Executor / Capabilities** | domain work | MCP, gateways |

**Runtime API Stability:** clients depend only on the Runtime API; planners, Executor, and capabilities may evolve behind it.

---

## Non-goals

- Embedding AudioPlanner / MeetingPlanner as the planning source of truth
- Executing Jobs or capability crates
- Scheduling / Resource Classes / stores
- Private wire protocol besides Runtime API
- CLI-named tools
- Gateway-owned durable artifact stores

---

## Tree (target)

```
src/cli/manage/vd-mcp/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── paths.rs
│   ├── cli/                     # serve · info · ping · doctor · config
│   ├── config/
│   ├── mcp/                     # session · tool registry · progress map
│   │   ├── tools/
│   │   │   ├── planning.rs      # process_audio · process_meeting (forward Requests)
│   │   │   ├── execution.rs     # submit_job · get_job · cancel · list_*
│   │   │   └── operator.rs      # health · doctor · server_info
│   │   └── progress.rs          # Event API → MCP notifications
│   ├── request/                 # Domain Request shapes + InputSource (see docs/input-source.md)
│   ├── client/                  # Runtime API Transport client (sole backend)
│   │   ├── submit.rs
│   │   ├── watch.rs
│   │   ├── query.rs
│   │   └── health.rs
│   └── error.rs
│
└── tests/
    ├── unit/
    ├── integration/             # mock Runtime API
    ├── e2e/
    └── fixtures/
```

No `planner/` module in this crate — planning is a Runtime concern.

Stub: [`docker/vd-mcp-stub.sh`](../../../../docker/vd-mcp-stub.sh).

---

## Module duties

| Module | Owns |
|--------|------|
| `mcp/tools/` | MCP schemas → Planning / Execution / Operator API |
| `request/` | Wire shapes; InputSource → [`docs/input-source.md`](../../../../docs/input-source.md) |
| `client/` | Runtime API only |
| `cli/doctor` | Gateway Doctor |
| `mcp/progress.rs` | Event API incl. `ArtifactConsumed` · `NodePhaseChanged` |

---

## Event API

`JobQueued` · `JobStarted` · `NodeQueued` · `NodeStarted` · `NodePhaseChanged` · `NodeProgress` · `ArtifactProduced` · `ArtifactConsumed` · `NodeCompleted` · `JobCompleted` | `JobFailed` | `JobCancelled`

---

## Dependencies (planned)

| Dependency | Use |
|------------|-----|
| Runtime API / Transport | **Only** backend |
| MCP SDK (TBD) | Host protocol |

**Must not** depend on capability crates or own Planner implementations.

---

## Tests (target)

| Layer | Focus |
|-------|--------|
| unit | Request shapes, `execute: false` mapping, event → progress |
| integration | Mock Runtime API |
| e2e | Live `vd-srv` |

```bash
cargo test -p vd-mcp
./scripts/test.sh vd-mcp
```

---

## Deployment

| Artifact | Role |
|----------|------|
| `vd-mcp` binary | MCP server process |
| `voxdecoder/mcp` | Optional container gateway |

N gateways → one Runtime. Stateless.
