# Runtime Environment

**Status:** living note  
**Related:** [ADR 0001](adr/0001-platform-refactoring-plan.md) · [ADR 0002 — Build & Container](adr/0002-build-and-container-strategy.md) · [ADR 0003 — Distribution & Update](adr/0003-distribution-and-update-strategy.md) · [`InputSource`](input-source.md) · [`vd-srv`](../src/cli/manage/vd-srv/) · [MCP](../src/cli/manage/vd-mcp/) · [`vdctl`](../src/cli/manage/vdctl/) (Platform CLI)

After Docker / `vd-srv` / Kubernetes, the platform has an explicit **Runtime Environment** and a stable **Runtime API**:

```text
Clients
  Desktop · CLI · MCP · REST · Web
        │
        ▼
Runtime API          ← public contract (stable)
  submit · cancel · subscribe · artifacts · health · …
        │
        ▼
Runtime (vd-srv)
  Planner · Scheduler · Resource Manager · Executor
        │
        ▼
Capabilities
  preprocess · transcribe · diarize · meeting · postprocess · …
```

| Role | What it does | Examples |
|------|----------------|----------|
| **Platform CLI** | Service lifecycle, platform doctor, assets, config, info — does not execute Jobs | **`vdctl`** |
| **Runtime API client** | Speaks only the Runtime API | **`vd-mcp`**, Desktop, Web UI, CLI `--via-srv`, REST/gRPC; `vdctl` for Operator observe |
| **Runtime** | Plans Domain Requests → Jobs; schedules; resources; observe; health; transport | **`vd-srv`** (Planner · Scheduler · Resource Manager · Executor host) |
| **Executor** | Runs the capability DAG / applies TimeMap | shared Executor (`vd-pipeline`) |
| **Capability** | Domain work | `vd-gigaam`, `vd-preprocess`, `vd-fix-*`, … |

**Runtime API Stability:** gateways and UIs depend only on the Runtime API. Planners, Executor, and capabilities may evolve without breaking clients.

Local CLIs (`vd-pipeline run`, `vd-meeting`) may still plan/run in-process for foreground use; when talking to a durable Runtime they become Runtime API clients like everyone else. [`vd-mcp`](../src/cli/manage/vd-mcp/) **only forwards Requests** — Planner implementations live in the Runtime.

### How capabilities are invoked

All VoxDecoder tools are included in the Runtime image.

The Runtime invokes capabilities through **shared libraries** where available,
falling back to **CLI subprocesses** for standalone tools (and for Desktop-local
workflows). CLI binaries stay thin wrappers around the same `lib` entrypoints:

```text
vd-gigaam (main) ──▶ run()
vd-srv     ───────▶ run()   (same implementation)
```

Subprocess remains a supported fallback while in-process binders land.

Scales the same way: local Runtime, Docker Runtime, Kubernetes Runtime, later a
distributed Runtime — **Job model unchanged**.

---

## Containers (by process role)

One **Dockerfile**, targets by `ENTRYPOINT` — not one container per binary.

**Production images (two):**

| Image | PID1 / binary | Role |
|-------|---------------|------|
| **`voxdecoder/runtime`** | `vd-srv serve` | Sole heavy image: Runtime + all capabilities (K8s worker) |
| **`voxdecoder/mcp`** | `vd-mcp` | Lightweight MCP gateway → Runtime API only |

**Optional:** `voxdecoder/dev` — developer / DevContainer (`vdctl`, toolchain). Not for production.

**No** `voxdecoder/vdctl` image. [`vdctl`](../src/cli/manage/vdctl/) is host-side; `docker run … voxdecoder/runtime` already replaces `vdctl up` inside containers.

```text
Native:     vdctl up  →  vd-srv
Container:  docker run voxdecoder/runtime  →  vd-srv serve
K8s:        Deployment  →  vd-srv
```

### `voxdecoder/runtime` (Worker)

```text
+------------------------------------+
| voxdecoder-runtime                 |
|------------------------------------|
| vd-srv (PID 1: serve)              |
| + tools / libs for capabilities    |
|   vd-pipeline · preprocess · …     |
|   ffmpeg · …                       |
+------------------------------------+
```

```bash
docker build -t voxdecoder/runtime --target runtime .
# optional: omit ffmpeg
docker build -t voxdecoder/runtime --target runtime --build-arg WITH_FFMPEG=0 .

docker run --rm -p 7701:7701 \
  -v "$PWD/models:/models:ro" \
  -v vd-data:/data \
  voxdecoder/runtime

# override listen (ENTRYPOINT stays `vd-srv serve`)
docker run --rm -p 7701:7701 voxdecoder/runtime --transport uds --socket /data/srv/vd-srv.sock
```

Default CMD: `--transport tcp --tcp 0.0.0.0:7701 --data-dir /data/srv`.  
`--workers` is **not** hardcoded — server default / Resource Classes apply; pass `--workers N` to override.

`HEALTHCHECK` runs `vd-srv ping` (config via `VD_SRV_CONFIG`).

### Layout inside the image

```text
/models
  gigaam/
  diarize/
  postprocess/

/data
  srv/          # VD_SRV_DATA — Job store, socket, pid
  cache/
  jobs/
  artifacts/
  logs/
  project/      # VD_PROJECT_DIR (.voxdecoder assets)

/work           # scratch / mounted workspaces
```

| Env | Default | Meaning |
|-----|---------|---------|
| `VD_MODELS_DIR` | `/models` | Shared models root |
| `VD_GIGAAM_MODELS_DIR` | `/models/gigaam` | GigaAM weights (also derived from `VD_MODELS_DIR`) |
| `VD_SRV_CONFIG` | `/etc/voxdecoder/runtime.toml` | Server + client endpoint defaults |
| `VD_SRV_DATA` | `/data/srv` | Durable Runtime state |
| `VD_PROJECT_DIR` | `/data/project` | Project knowledge pack |

### `voxdecoder/mcp` (interface only)

```text
Claude / Cursor / VS Code
        ↓
     vd-mcp          ← MCP Gateway (Runtime API client only)
        ↓
     vd-srv          ← Runtime (separate deployment)
```

MCP does **not** execute Jobs, does **not** need GPU, and may be omitted. It speaks
**Transport** (same as other clients), not HTTP:

```text
VD_TRANSPORT=tcp
VD_TCP=runtime:7701

# or
VD_TRANSPORT=uds
VD_SOCKET=/tmp/vd.sock
```

See [`vd-mcp`](../src/cli/manage/vd-mcp/).

```bash
docker build -t voxdecoder/mcp --target mcp .
```

### Desktop and `vdctl`

No production container. Desktop and [`vdctl`](../src/cli/manage/vdctl/) run on the host.

* Local: `vdctl up` → spawn `vd-srv` (Workspace or Installed), or attach if already up.
* Remote: optional endpoint overrides for Operator calls; Docker/K8s remain Runtime details — same `vdctl` commands locally.

Transport: [TRANSPORT.md](../src/cli/manage/vd-srv/TRANSPORT.md).

---

## Kubernetes sketch

```text
           Ingress
               │
         Transport / API
               │
      +----------------+
      |  Runtime Pod   |   ← voxdecoder/runtime × N
      +----------------+
          Worker Pool
```

Scale by identical Runtime replicas. Optional MCP Deployment only when IDE clients need it.

---

## Build

```bash
docker build -t voxdecoder/runtime --target runtime .
docker build -t voxdecoder/mcp --target mcp .
# optional later: docker build -t voxdecoder/dev --target dev .
docker compose build runtime && docker compose up runtime
npm run build   # native binaries incl. vdctl when wired (scripts/build.sh)
```
