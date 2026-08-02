# ADR: Build & Container Strategy

**Status:** Accepted  
**Type:** ADR  
**Date:** 2026-08-02

## Goal

Define a single build strategy for VoxDecoder that supports:

* native development on macOS, Linux, and Windows;
* platform-specific acceleration (Metal, CUDA, …);
* Docker images;
* Kubernetes deployments;
* reproducible CI builds.

---

## Principles

### One source tree

All binaries are built from the same workspace.

There are no platform-specific forks.

Platform differences are implemented through:

* Cargo features
* conditional compilation
* runtime backend selection

— not separate codebases.

### Native builds and container builds are different products

Native builds target the host operating system.

Container builds target Linux only.

This distinction is intentional.

```text
Native Build
    ↓
macOS
Linux
Windows

Docker Build
    ↓
Linux
```

Container images are never expected to provide Metal support.

---

## Runtime Backends

Every compute-heavy capability should expose the same runtime abstraction.

```text
Runtime Backend

auto
cpu
metal
cuda
rocm
```

Example:

```yaml
backend:
  type: auto
```

or

```bash
vd-gigaam run --backend metal
# today: --device metal | cuda | cpu | auto  (unify naming over time)
```

`auto` selects the best backend available on the current machine.

Typical resolution:

```text
macOS Apple Silicon  →  Metal
Linux + NVIDIA       →  CUDA
Linux                →  CPU
Windows              →  CPU
```

The backend is a **runtime** concern. It is not part of Job semantics.

---

## Cargo features

Platform acceleration is enabled through Cargo features.

Example:

```toml
[features]
default = []
metal = ["candle-core/metal", "candle-nn/metal"]
cuda = []   # when wired
rocm = []   # when wired
```

Each capability enables only the features it needs.

---

## Conditional compilation

Platform-specific code must remain isolated.

```rust
#[cfg(feature = "metal")]
#[cfg(target_os = "macos")]
```

Business logic should remain platform-independent.

---

## Build script

[`scripts/build.sh`](../../scripts/build.sh) detects the current platform automatically.

```text
macOS   →  cargo build --release … --features vd-gigaam/metal
Linux   →  cargo build --release …
Windows →  cargo build --release …   (via CI / native toolchain)
Docker  →  ./scripts/build.sh --cpu   (Linux, no Metal)
```

Developers should not need to remember platform-specific feature flags.

```bash
npm run build          # host defaults (Metal on macOS)
npm run build:cpu      # force CPU features (same as Docker)
```

---

## GitHub Actions

CI builds every supported platform independently.

Example matrix:

```text
Ubuntu  →  cargo build --release  (via scripts/build.sh --cpu)
macOS   →  cargo build --release  (Metal via scripts/build.sh)
Windows →  cargo build --release
```

Platform-specific feature selection happens inside each job. This produces native
binaries for every supported platform.

---

## Docker images

Docker images are **Linux** runtime environments.

Metal is never expected inside a container.

**Canonical images:**

```text
voxdecoder/runtime     ← production worker (vd-srv + capabilities); K8s
voxdecoder/mcp         ← lightweight MCP gateway only
voxdecoder/dev         ← optional DevContainer (vdctl + toolchain); not production
```

There is **no** production `voxdecoder/vdctl` image. [`vdctl`](../src/cli/manage/vdctl/) is host-side Platform CLI; container PID1 is already `vd-srv serve`.

Future **variants** of the runtime image (same contract, different deps):

```text
voxdecoder/runtime-cpu
voxdecoder/runtime-cuda
```

All Runtime variants share the same Job / Runtime API contract. Only native deps (CUDA libs, …) differ.

See [`docs/runtime.md`](../runtime.md).

---

## Container philosophy

A container hosts one Runtime.

```text
Container
    ↓
vd-srv
    ↓
Executor
    ↓
Capabilities
```

Capability binaries are included in the image.

The Runtime invokes them through shared libraries where available, with CLI
subprocesses as a compatibility fallback.

---

## Runtime model cache

Model assets are stored outside the container image.

```text
/models
  gigaam/
  diarize/
  preprocess/
  postprocess/
```

```text
VD_MODELS_DIR=/models
```

Capabilities resolve their own subdirectory (`vd-gigaam` → `/models/gigaam`, …).

---

## Runtime data

Persistent runtime state is separated from binaries.

```text
/data
  srv/
  jobs/
  artifacts/
  cache/
  logs/
```

Mount as a persistent volume in Docker and Kubernetes.

---

## Kubernetes

Every replica runs exactly the same Runtime.

```text
Deployment
  Replica → vd-srv
  Replica → vd-srv
```

Scaling = more Runtime replicas. No capability-specific containers.

---

## MCP

The MCP server is deployed separately (`voxdecoder/mcp`).

```text
Client → vd-mcp → vd-srv
```

MCP is an interface. It does not execute Jobs. It uses the standard Transport layer.

## Platform CLI (`vdctl`)

Native install / Desktop companion. Spawns or attaches to Runtime on the host; may target a remote Runtime API endpoint (Docker TCP, Kubernetes Service). Does not replace container ENTRYPOINT and is not a third production image.

---

## Transport

Transport is independent of deployment.

Supported: Unix Domain Socket · Named Pipe · TCP.

| Context | Typical transport |
|---------|-------------------|
| Desktop | UDS / Named Pipe |
| Containers / k8s | TCP |

The Runtime API remains identical.

---

## Build matrix

| Platform | Native | Docker | Backend |
|----------|--------|--------|---------|
| macOS Apple Silicon | ✅ | ❌ | Metal |
| macOS Intel | ✅ | ❌ | CPU |
| Linux x86_64 | ✅ | ✅ | CPU |
| Linux + CUDA | ✅ | ✅ | CUDA |
| Windows | ✅ | ❌ | CPU |

Future backends (ROCm, DirectML, …) extend this table without changing the Job model.

---

## Guarantees

The build system guarantees:

* one workspace;
* one Runtime architecture;
* one Job model;
* one Executor;
* platform-specific acceleration hidden behind Runtime Backends;
* identical Job behavior between native and container execution;
* Docker images remain Linux-only;
* Metal support remains a native macOS feature.

---

## Related

* [`scripts/build.sh`](../../scripts/build.sh)
* [`Dockerfile`](../../Dockerfile)
* [`docs/runtime.md`](../runtime.md)
* [ADR 0001 — Platform Refactoring](0001-platform-refactoring-plan.md)
* [ADR 0003 — Distribution & Update](0003-distribution-and-update-strategy.md)
* [`vdctl`](../../src/cli/manage/vdctl/)
* [`vd-mcp`](../../src/cli/manage/vd-mcp/)
* [`vd-gigaam` features](../../src/cli/transcribe/vd-gigaam/Cargo.toml)
