# Runtime Environment

**Status:** living note  
**Related:** [ADR 0001](adr/0001-platform-refactoring-plan.md) · [ADR 0002 — Build & Container](adr/0002-build-and-container-strategy.md) · [`vd-srv`](../src/cli/manage/vd-srv/) · [MCP](../src/cli/manage/vd-mcp/)

After Docker / `vd-srv` / Kubernetes, the platform has an explicit **Runtime Environment**:

```text
              Container / host
                      │
                      ▼
                   Runtime
                    (vd-srv)
                      │
         Worker Pool · Resource Classes · Queue
         Event Store · Artifact Store · Health
         Transport · API · Scheduling
                      │
                      ▼
                   Executor
                      │
                      ▼
                 Capabilities
```

| Role | What it does | Examples |
|------|----------------|----------|
| **Builder** | Constructs a Job | `vd-pipeline` CLI, `vd-meeting`, `vd-mcp`, Desktop, HTTP clients |
| **Runtime** | Job lifecycle, schedule, resources, observe, health, transport | **`vd-srv`** |
| **Executor** | Runs the capability DAG / applies TimeMap | shared Executor (`vd-pipeline`) |
| **Capability** | Domain work | `vd-gigaam`, `vd-preprocess`, `vd-fix-*`, … |

`vd-pipeline` is both a **Builder** (CLI Job builder) and the home of the **Executor** library. Locally it can run a Job without Runtime (`vd-pipeline run`).

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

One **Dockerfile**, images by `ENTRYPOINT` — not one container per binary.

### `voxdecoder/runtime` (Worker)

```text
+------------------------------------+
| voxdecoder-runtime                 |
|------------------------------------|
| vd-srv (PID 1: serve)              |
| + tools / libs for capabilities    |
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
     vd-mcp          ← this image (Transport client)
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

### Desktop

No container. Desktop talks to a local Runtime over UDS / named pipe
([TRANSPORT.md](../src/cli/manage/vd-srv/TRANSPORT.md)).

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
docker compose build runtime && docker compose up runtime
npm run build   # same binary set locally (scripts/build.sh)
```
