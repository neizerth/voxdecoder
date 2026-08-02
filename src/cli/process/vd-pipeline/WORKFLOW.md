# Workflow Executor + Artifacts (Epics 1–8)

Living RFC for the roadmap. **Out of scope here:** `vd-srv`, MCP.

## Epic 1 — status: implemented (MVP)

Job `steps` is a list of [`WorkflowNode`](src/job/schema.rs):

| Node | Wire shape |
|------|------------|
| Capability leaf | `{ use: transcribe, … }` |
| Sequence | `{ sequence: [ …nodes ] }` |
| Parallel | `{ parallel: [ …nodes ] }` |

Flat lists of capability steps remain valid (implicit root sequence).

Executor walks `WorkflowPlan` recursively. `parallel` runs children in `thread::scope` batches of `max_parallel`. Branch progress is quiet; parent report still collects leaf timings.

## Epic 2 — status: scaffold

- `Step.produces` / `Step.consumes` (optional; fall back to `id` / `inputs`).
- [`ArtifactRegistry`](src/artifacts.rs): named insert, wildcard `prefix/*`, merge across parallel branches.
- Capability `default_artifact_kind()` hints (`transcript`, `timeline`, `meeting`, …).

## Epic 3 — status: partial

`vd-meeting` Planner emits a workflow tree: optional `prepare-context`, **parallel transcript branches** when multiple text sources, then diarize + merge leaves. Default `max_parallel: 4` when unset.

## Epics 4–7 — status: stubs remain; contracts

| Epic | Capability | Artifact kind | Notes |
|------|------------|---------------|-------|
| 3b | `preprocess` | `media` / prepared | Filter chain; default Job head; per-branch in meetings — see [`vd-preprocess`](../vd-preprocess/) |
| 4 | `diarize` | `timeline` | Real backends TBD; stub binder stays |
| 5 | `meeting-merge` | `meeting` | Alignment strategies TBD |
| 6 | Meeting export | `meeting` | Multi-format export TBD |
| 7 | `postprocess` | `derived` | Providers TBD |

## Epic 8 — status: extended report

`ExecutionReport` includes `queued_at` / `started_at` / `finished_at` per step, plus `critical_path_ms` (max leaf duration) and `parallel_efficiency` (`work_sum / wall`). Full critical-path over the workflow DAG is follow-up.

## Example

```yaml
version: 1
max_parallel: 2
input:
  audio: meeting.ogg
steps:
  - use: transcribe
    id: transcript
  - parallel:
      - use: fix-casing
        input: transcript
      - use: diarize
```
