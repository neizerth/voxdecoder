# ADR 0008 — Input Resolution Layer

**Status:** Proposed  
**Type:** ADR  
**Date:** 2026-08-03

**Related:** [`vd-pipeline`](../../src/cli/process/vd-pipeline/) · [`vd-meeting`](../../src/cli/process/vd-meeting/) · [`vd-url`](../../src/cli/process/vd-url/) · [`vd-srv`](../../src/cli/manage/vd-srv/) · [`vd-mcp`](../../src/cli/manage/vd-mcp/) · [`vd-input`](../../src/crates/vd-input/) · [`docs/input-source.md`](../input-source.md)

---

## Motivation

Planners today accept different kinds of inputs:

- local files
- artifact references
- URLs

Future sources may include Vimeo, RuTube, VK Video, archives, and cloud storage.

Without a dedicated resolution layer, every Planner implements its own input handling. That duplicates logic and makes default pipelines **dynamic** (shape depends on the original source).

---

## Problem

The default pipeline changes with the input:

```text
file  →  preprocess  →  transcribe  →  …

url   →  import-url  →  preprocess  →  transcribe  →  …
```

Later, a Subtitle Artifact might skip ASR entirely. Planners grow increasingly source-aware.

---

## Goal

Separate **input resolution** from **Job planning**.

The Planner always receives Runtime artifacts. It must not care whether those artifacts came from a local file, a URL, another Job, or a future provider.

---

## Core rule

```text
Input resolution happens before Job planning.

Planners consume artifacts.

They do not consume user sources.
```

---

## Architecture

```text
User Input
    │
    ▼
InputSource
    │
    ▼
Input Resolver   (vd-input)
    │
    ▼
ResolvedInput (Artifacts)
    │
    ▼
Planner
    │
    ▼
Job
    │
    ▼
Executor
```

---

## Decision

### Shared crate `vd-input`

Path: `src/crates/vd-input/`.

Owns:

- `InputSource` parsing / XOR validation
- provider / resolver selection
- artifact materialization
- metadata propagation into `ResolvedInput`

Does **not** own:

- Job construction
- scheduling
- transcription, preprocess, diarize, postprocess

### `ResolvedInput`

Resolution yields Runtime-ready artifact paths (audio, metadata, optional subtitle, …). Planners work only with these.

### Provider model

```text
resolve(InputSource, ResolveContext) → ResolvedInput
```

v1 resolvers:

| Source | Resolver |
|--------|----------|
| `path` / `file://` | `FileResolver` |
| `url` | `UrlResolver` (shared with `vd-url`) |
| `artifact` | `ArtifactResolver` |
| `blob` | `BlobResolver` |

### `vd-url`

CLI over the same `UrlResolver`. One implementation for CLI and Runtime.

### Planning

After resolution the default audio pipeline is **static**:

```text
ResolvedInput.audio
  → preprocess → transcribe → fix-* → …
```

`import-url` is **not** inserted by `default_job` / meeting DAG builders.

### Runtime / MCP / Meeting

Planning API calls `vd-input` before domain planners. Meeting inputs are resolved per entry, then the meeting DAG is built from local artifact paths.

---

## Consequences

**Positive**

- Planners stay source-agnostic and simpler
- Default Jobs are deterministic
- New providers = new Resolver only
- Frontends do not resolve inputs independently

**Trade-offs**

- URL / remote sources materialize during Planning (before Job submit). Plan-only with a live URL may download (or use a stub provider in tests).
- `Capability::ImportUrl` remains for explicit Jobs / CLI; planners no longer emit it by default.

---

## Success criteria

- [x] Shared `vd-input` crate
- [x] Planners consume `ResolvedInput` (paths), not raw URLs
- [x] URL import removed from `default_job` construction
- [x] `vd-url` uses the same UrlResolver library
- [x] Default audio pipeline independent of original source kind
- [ ] Future providers (Vimeo, …) without Planner changes

---

## Status notes

Proposed → implement in-tree; promote to **Accepted** once Runtime + MCP docs and skills are aligned.
