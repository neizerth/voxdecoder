# VoxDecoder — Platform Refactoring Plan

**Status:** proposed  
**Type:** ADR / RFC  
**Date:** 2026-08-02

This document describes architectural changes required before implementing the next
generation of VoxDecoder capabilities (`vd-preprocess`, DAG execution, `vd-meeting`,
`vd-srv`, desktop UI).

It is not about a single CLI — it describes platform-wide changes.

---

## Goals

The platform is evolving from a **linear pipeline** into a **general artifact-processing platform**.

Required architectural changes:

* DAG execution instead of linear chains
* parallel execution
* resource scheduling
* preprocessing support
* canonical timeline handling
* richer artifact model
* reusable execution graph

---

## 1. DAG Executor

### Current

```text
step1
  ↓
step2
  ↓
step3
```

Execution is strictly linear.

### Target

The Executor executes a DAG.

```text
          A
         / \
        B   C
         \ /
          D
```

A node starts when:

* all dependencies completed
* required resources are available

Independent branches execute concurrently.

### Required changes

Job schema:

```yaml
steps:
  - id: transcript
    use: transcribe
  - id: casing
    needs: [transcript]
    use: fix-casing
  - id: summary
    needs: [casing]
    use: postprocess
```

Executor:

* dependency resolution
* ready queue
* scheduler
* parallel workers

> **Note:** `vd-pipeline` already has workflow `sequence` / `parallel`, `depends` /
> `consumes`, Kahn topo ordering, and `max_parallel`. Further work completes ready-queue
> scheduling and unifies Job sugar (`needs` ↔ `depends` / `consumes`).

---

## 2. Parallel execution

Executor must support configurable concurrency.

Example:

```text
Alice
        \
Bob ------ merge
        /
Merged diarize
```

All transcript branches run simultaneously.

New Job option:

```yaml
max_parallel: 4
```

Later replaced by Resource Manager in `vd-srv`.

---

## 3. Resource-aware scheduling

Capabilities declare required resources.

Example:

```yaml
resources:
  gpu: 1
  cpu: 2
  llm: 1
```

Executor waits until resources are available.

Future pools: CUDA, Metal, CPU, RAM, LLM, Network.

---

## 4. Preprocessing becomes a capability

```text
use: preprocess
```

```text
Audio → preprocess → Audio' → transcribe
```

Implemented as `vd-preprocess` (filter graph) bound from the Job Executor.

---

## 5. TimeMap Artifact

This is the biggest architectural addition.

### Problem

Some preprocessing changes time:

* `speed`
* `trim-silence`
* future VAD compression

Those transformations invalidate timestamps from ASR, diarization, and subtitles.

### Approach

Instead of teaching every CLI how to compensate time, `vd-preprocess` produces another
artifact:

```text
Audio → Preprocess → Audio' + TimeMap
```

TimeMap maps **processed timeline → original timeline**.

### Artifact

```text
ArtifactType / kind: TimeMap
```

Example:

```yaml
version: 1
segments:
  - processed: { start: 0, end: 20 }
    original:  { start: 0, end: 25 }
  - processed: { start: 20, end: 40 }
    original:  { start: 30, end: 50 }
```

Constant `speed` is a single segment covering the full utterance
(`processed_end = original_end / factor`).

---

## 6. Executor applies TimeMap

Capabilities operate only on their input. They never know preprocessing happened.

```text
Audio' → transcribe → Transcript (processed timeline)
```

Executor detects that the transcript depends on a TimeMap and remaps
**processed → original** before registering canonical artifacts.

Same for: diarization, subtitles, meeting merge, chapter markers, any future
timeline artifact.

---

## 7. Timeline-capable artifacts

Artifacts containing timestamps become explicit:

* Transcript (segments / words / SRT / VTT)
* SpeakerTimeline
* Subtitle
* Meeting
* Chapters
* Bookmarks

Executor remaps them via TimeMap. No capability-specific remap code.

---

## 8. Meeting Planner

Meeting Planner builds DAGs (participant branches + merged diarize → meeting-merge).

---

## 9. Postprocess

Postprocess is another DAG:

```text
Pipeline DAG → Recipe Graph → Execution Graph
```

Same Executor model at the orchestration layer; recipe nodes run via `ExecutionRunner`.

---

## 10. Shared execution model

Three execution layers:

```text
Media
  ↓
Filter Graph (vd-preprocess)
  ↓
Artifacts
  ↓
Capability DAG (vd-pipeline)
  ↓
Artifacts
  ↓
Recipe Graph (vd-postprocess)
  ↓
Derived Artifacts
```

Each layer owns its own abstraction.

---

## 11. Executor responsibilities

* DAG scheduling
* artifact resolution
* dependency tracking
* resource scheduling
* TimeMap application
* progress aggregation
* metrics
* artifact registration

Capabilities remain unaware of orchestration.

---

## 12. Required artifact kinds

```text
Audio
Transcript
SpeakerTimeline
Meeting
TimeMap
ExecutionPlan
RecipeResult
DerivedArtifact
```

---

## 13. Future compatibility

These changes enable `vd-meeting`, `vd-srv`, desktop UI, MCP, distributed execution,
preprocessing chains, multiple timeline transforms, and nested recipe graphs — without
changing capability interfaces.

---

## 14. Unified Artifact Model

The platform is no longer “a set of CLIs”. It is a system that transforms artifacts.

Every capability is one formula:

```text
Artifact(s)
      ↓
Capability
      ↓
Artifact(s)
```

`vd-preprocess`, `vd-pipeline`, `vd-postprocess`, and `vd-meeting` are different ways to
**build a graph of transformations between artifacts**. That is the central architectural
idea of the platform.

| Layer | Graph unit | Graph product |
|-------|------------|---------------|
| `vd-preprocess` | Filter | Prepared media (+ TimeMap when time changes) |
| `vd-pipeline` | Capability | Domain artifacts (transcript, timeline, …) |
| `vd-postprocess` | Recipe node | Derived artifacts |
| `vd-meeting` | Planner | Job (Capability DAG) |

---

## 15. Runtime Environment

After `vd-srv` / Docker / Kubernetes, the platform has an explicit **Runtime**:

```text
Container / host → Runtime (vd-srv) → Executor → Capabilities
```

| Role | Responsibility | Examples |
|------|----------------|----------|
| **Builder** | Construct a Job | `vd-pipeline` CLI, `vd-meeting`, `vd-mcp`, Desktop, HTTP |
| **Runtime** | Lifecycle, Worker Pool, Resource Classes, Queue, Event/Artifact store, Health, Transport, API, Scheduling | **`vd-srv`** |
| **Executor** | Execute the capability graph | shared Executor (`vd-pipeline`) |
| **Capability** | Domain work | preprocess, gigaam, fix-*, … |

Capabilities are invoked via **shared libraries** where available, with **CLI
subprocess fallback**. CLI binaries stay thin wrappers around the same `run()`.

`vd-pipeline` remains a Builder (and hosts the Executor for foreground `run`
without Runtime). See [`docs/runtime.md`](../runtime.md).

### Containers

One Dockerfile, images by process role:

* **`voxdecoder/runtime`** — tools + `ENTRYPOINT ["vd-srv","serve"]` (TCP in `CMD`, overridable)
* **`voxdecoder/mcp`** — MCP interface only (Transport client; optional; no GPU)
* **Desktop** — no container; local UDS/pipe → Runtime

---

## Resulting architecture

```text
                     Media
                       │
                       ▼
              Filter Graph
             (vd-preprocess)
                       │
             Audio + TimeMap
                       │
                       ▼
             Capability DAG
              (vd-pipeline)
                       │
                 Artifacts
                       │
             Executor applies
                 TimeMap
                       │
                       ▼
              Canonical Artifacts
                       │
                       ▼
               Recipe Graph
            (vd-postprocess)
                       │
                       ▼
             Derived Artifacts
```

---

## Implementation status (living)

| Item | Status |
|------|--------|
| §1–2 DAG + `max_parallel` | partial (`sequence`/`parallel`/`depends`) |
| §3 Resource manager | not started |
| §4 `use: preprocess` | done |
| §5–7 TimeMap + executor remap | **done (first cut)**: speed → uniform TimeMap sidecar; executor remaps segments/words/SRT after transcribe/diarize |
| §8 Meeting planner DAGs | partial |
| §9 Postprocess recipe graph | done (first cut) |
| §14 Unified model (docs + kinds) | this ADR |
| §15 Runtime Environment + containers | **docs + Dockerfile** (`runtime`/`mcp`; HEALTHCHECK; `/data` + `/models` layout; lib-first invoke is the target, subprocess fallback) |

---

## Related

* [docs/runtime.md](../runtime.md)
* [ADR 0002 — Build & Container Strategy](0002-build-and-container-strategy.md)
* [`vd-mcp`](../../src/cli/manage/vd-mcp/)
* [src/cli/process/README.md](../../src/cli/process/README.md)
* [vd-pipeline WORKFLOW.md](../../src/cli/process/vd-pipeline/WORKFLOW.md)
* [vd-preprocess README](../../src/cli/process/vd-preprocess/README.md)
