# ADR 0003 — Distribution & Update Strategy

**Status:** Proposed  
**Type:** ADR  
**Date:** 2026-08-02

**Related: [`vdctl`](../../src/cli/manage/vdctl/) · [`vd-srv`](../../src/cli/manage/vd-srv/) · [`vd-mcp`](../../src/cli/manage/vd-mcp/) · [`docs/runtime.md`](../runtime.md) · [ADR 0002 — Build & Container](0002-build-and-container-strategy.md) · [ADR 0005 — MCP Bundle & Skills](0005-mcp-bundle-and-skill-distribution.md)

---

## Goal

Define a single distribution and update strategy for the VoxDecoder platform.

The platform should support:

* local desktop installations;
* developer installations;
* CI;
* Docker;
* Kubernetes;

without introducing different installation models.

---

## Core rule

```text
GitHub Releases distribute native binaries.

GitHub Container Registry distributes containers.

vdctl is the manager of the local installation:
  install · update · uninstall · verify · assets.

Capabilities never update themselves.

A development Workspace is never updated from Releases.
```

`vdctl` is **not** a binary, process, or context manager — it manages the **installation** as a whole. See [`vdctl` README](../../src/cli/manage/vdctl/README.md).

---

## Distribution channels

The platform has three independent distribution channels.

```text
                GitHub

          ┌────────┴────────┐
          │                 │
      Releases           GHCR
          │                 │
          ▼                 ▼
   Native binaries     Containers
          │                 │
          ▼                 ▼
        vdctl         Docker / k8s
```

| Channel | Artifact | Consumer |
|---------|----------|----------|
| **GitHub Releases** | Platform archives (all native binaries) | `vdctl` / Desktop / brew·winget·curl installers |
| **GHCR** | `voxdecoder/runtime`, `voxdecoder/mcp` (+ optional `dev`) | Docker / Kubernetes |
| **Assets** (separate) | Models, packs, dictionaries, … | `vdctl assets` |

There is no production `voxdecoder/vdctl` image — see [ADR 0002](0002-build-and-container-strategy.md) and [`vdctl` README](../../src/cli/manage/vdctl/README.md).

---

## Native installation

Native installations use **GitHub Releases**.

Each Release contains archives for every supported platform.

Example:

```text
v1.2.0

Assets

voxdecoder-macos-aarch64.tar.gz
voxdecoder-linux-x86_64.tar.gz
voxdecoder-windows-x64.zip

checksums.txt
manifest.json
```

---

## Installed binaries

A release archive contains the complete Runtime / platform set — not one-capability downloads.

Example:

```text
vdctl
vd-srv
vd-mcp

vd-pipeline
vd-preprocess
vd-postprocess
vd-meeting
vd-diarize

vd-gigaam

vd-fix-casing
vd-fix-asr
vd-fix-terms
```

No binary downloads itself independently.

---

## Platform lifecycle (`vdctl`)

Primary commands (Installed mode):

```bash
vdctl install
vdctl update
vdctl uninstall
vdctl uninstall --purge
```

| Command | Behavior |
|---------|----------|
| **`install`** | Download Release → install binaries → create dirs → register `vdctl` → offer MCP registration |
| **`update`** | Check Releases → download → replace **whole platform** → restart Runtime if needed → compatible assets / migrations |
| **`uninstall`** | Remove platform; prompt to keep data (or `--purge`) |

Check / version:

```bash
vdctl update --check     # or vdctl doctor compatibility
vdctl version
```

Future: `vdctl update --rollback`.

### Update flow

```text
vdctl update
  ↓
GitHub Releases → manifest
  ↓
compare versions → download → verify checksum
  ↓
replace binaries (atomic / restore on failure)
  ↓
restart Runtime if needed
  ↓
assets / migrations if required
```

### Workspace mode

If `Cargo.toml` is found (Workspace), **install / update / uninstall from Releases are refused**:

```text
Running from a development workspace.

Updates are managed through Git.
Use:

    git pull
    cargo build
```

Runtime still starts with `vdctl up` from `target/debug` (optional `cargo build` via `auto_build`).

### Update policy

Default: **manual**. Runtime never auto-updates itself.

```toml
[updates]
check_on_start = true
auto_download = false
channel = "stable"
```

Channels: `stable` · `beta` · `nightly`

```bash
vdctl update --channel stable
vdctl update --channel beta
```

---

## Manifest

Every Release publishes a machine-readable manifest (do not scrape GitHub HTML).

Example:

```json
{
  "version": "1.2.0",
  "api_version": "1",
  "minimum_runtime": "1.1.0",
  "artifacts": {
    "macos-aarch64": "…",
    "linux-x86_64": "…",
    "windows-x64": "…"
  }
}
```

Purpose:

* update discovery
* compatibility checks
* installer automation

---

## Compatibility

`vdctl doctor` reports:

```text
CLI Version
Runtime Version
API Version
Compatible ✔
```

If incompatible:

```text
Update required.
```

Doctor may suggest `vdctl update` when the active channel has a compatible newer release (Installed mode only).

---

## Assets

Platform binaries and downloadable **assets** are separate concepts.

```text
Platform
    │
    ├── binaries      ← GitHub Releases / vdctl install · update
    └── assets        ← vdctl assets …
```

Assets include:

* ASR models
* diarization models
* DeepFilterNet
* language packs
* recipe packs
* dictionaries
* prompt packs

```bash
vdctl assets list
vdctl assets install
vdctl assets update
vdctl assets remove
```

Updating assets **never** replaces platform binaries.

---

## Docker distribution

Containers are distributed through **GitHub Container Registry**.

```text
ghcr.io/<org>/voxdecoder/runtime:1.2.0
ghcr.io/<org>/voxdecoder/mcp:1.2.0
```

Docker images are immutable. Updates use standard pull semantics:

```bash
docker pull ghcr.io/<org>/voxdecoder/runtime:latest
```

Image layout / ENTRYPOINT: [`docs/runtime.md`](../runtime.md) · [ADR 0002](0002-build-and-container-strategy.md).

---

## Kubernetes

Kubernetes always consumes **GHCR** images — never GitHub Release archives.

```yaml
image: ghcr.io/<org>/voxdecoder/runtime:1.2.0
```

---

## Desktop (future)

Desktop never downloads binaries itself.

Instead it invokes:

```bash
vdctl install
vdctl update
vdctl uninstall
```

or the corresponding shared platform library.

One installation-management implementation for CLI and Desktop.

---

## Installation (bootstrap)

Canonical path:

```bash
vdctl install
```

Package managers may wrap the same Release artifacts (same post-install behavior):

| Platform | Example |
|----------|---------|
| macOS | `brew install voxdecoder` |
| Windows | `winget install VoxDecoder` |
| Linux | `curl … \| sh` → often boots into `vdctl install` |

Afterwards: `vdctl doctor`, `vdctl up`, `vdctl update`, `vdctl uninstall`.

---

## Verification

Downloaded binaries must be verified.

At minimum:

```text
SHA256
```

Future:

```text
cosign
# or
minisign
```

---

## Failure handling

If an update fails:

```text
keep previous binaries
restore automatically
report failure
```

No partially updated installation should remain.

---

## Non-goals

This strategy intentionally excludes:

* automatic Runtime updates (default)
* self-updating capabilities
* updating a development Workspace from Releases
* Docker image mutation in place
* divergent platform-specific update logic outside `vdctl` / shared libs
* a `service` CLI vocabulary for Runtime lifecycle (`up` / `down` instead)

Installer packaging (Homebrew, winget, …) remains an implementation detail on top of Releases.

---

## Success criteria

```bash
vdctl install
vdctl update
vdctl uninstall
vdctl assets update
vdctl doctor
vdctl up
```

Users need not understand GitHub Releases, Docker images, or Runtime internals.

Workspace developers use Git + `vdctl up` only — not Release-based install/update.

---

## Future extensions

Without changing this architecture:

* Homebrew Tap · Winget · Scoop · Chocolatey
* APT / RPM repositories
* Sparkle (macOS Desktop) · Windows Installer
* Auto-update notifications
* Signed releases
* Delta (binary diff) updates

The public contract remains unchanged:

```text
GitHub Releases  →  native binaries
GHCR             →  container images
vdctl            →  install · update · uninstall · verify · assets
```

---

## Related

* [`vdctl`](../../src/cli/manage/vdctl/) · [cli.md](../../src/cli/manage/vdctl/cli.md)
* [ADR 0001 — Platform Refactoring](0001-platform-refactoring-plan.md)
* [ADR 0002 — Build & Container](0002-build-and-container-strategy.md)
* [`docs/runtime.md`](../runtime.md)
