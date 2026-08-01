# vd-pipeline — universal Job Executor

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI / Job surface: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [vd-meeting](../vd-meeting/) · [vd-diarize](../vd-diarize/) · [../../fix/README.md](../../fix/README.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented** (linear Jobs today; DAG / named outputs / parallelism — schema + Executor contract, rolling out).  
Workspace member: `src/cli/process/vd-pipeline`.

## Core rule

```text
vd-pipeline is the universal Job Executor.

CLI, vd-meeting, vd-srv, and MCP only build or submit Jobs.
The Executor runs a DAG of capabilities — it does not care who authored the Job.
```

```text
CLI flags
Job file
vd-meeting (Job Builder)
MCP JSON
vd-srv

        ↓

       Job  (DAG of steps + named artifacts)

        ↓

    Executor  (schedule · parallel · resource limits)

        ↓

  Capabilities  →  implementations
```

| Layer | Role |
|-------|------|
| **Job builders** (`vd-pipeline` CLI, [`vd-meeting`](../vd-meeting/), MCP, `vd-srv`) | UX / domain → Job document |
| **Job** (YAML/JSON) | DAG of capabilities + named artifacts |
| **Executor** | Schedules ready steps, binds capabilities, registers artifacts |

The binary is still named `vd-pipeline` for familiarity; the product is the **Executor**.

## Capabilities (`use`)

`use` names an **action**, not a binary:

| `use` | Meaning | Implementation |
|-------|---------|----------------|
| `transcribe` | Audio/video → transcript | `engine: gigaam` (default), `whisper` (**reserved**) |
| `prepare-context` | Build project context (`.voxdecoder`) | `vd-assets` |
| `fix-casing` | Presentation | `vd-fix-casing` |
| `fix-asr` | Wording / ASR repair | `vd-fix-asr` |
| `fix-terms` | Canonical terminology | `vd-fix-terms` |
| `diarize` | Who spoke when | [`vd-diarize`](../vd-diarize/) |
| `meeting-merge` | Build Meeting Artifact from transcripts + timeline | stub binder in [`vd-pipeline`](.) (real merge later) |

Knobs live under `options:`. Reserved step fields stay free (`id`, `name`, `inputs`, `outputs`, `depends`, …).

## DAG

Steps form a **directed acyclic graph**, not only a list:

```text
        step1
       /     \
      v       v
   step2    step3
      \       /
       v     v
        step4
```

- Edges come from **`inputs`** (artifact dependencies) and optional **`depends`** (ordering without data).
- Independent ready steps may run **concurrently**.
- Executor limits concurrency via **`max_parallel`** and **resource groups** (`gpu` / `cpu` / `io`).

A classic linear cleanup Job is still a valid DAG (one chain).

## Named artifacts

| Field | Role |
|-------|------|
| `id` | Registers this step’s **primary** output as a named artifact |
| `outputs` | Additional named outputs (`md`, `terms`, `segments`, …) |
| `name` | Optional human label for UI / logs / progress — **never** for wiring |
| `inputs` | Artifact ids or paths this step consumes |
| `input` | Sugar for a single-entry `inputs` |

Downstream steps reference artifacts by name — they do not hard-code filesystem paths.

```yaml
- use: transcribe
  id: transcript

- use: fix-casing
  inputs:
    - transcript
```

## Default Job (from CLI)

```bash
vd-pipeline run -i meeting.ogg
vd-pipeline run -i meeting.ogg --asr gigaam -m v2_rnnt --docs ./docs
```

builds the **same** Job a file would describe (linear transcribe → fix-*), then submits it to the Executor.

[`vd-meeting`](../vd-meeting/) builds larger DAGs (N track cleanups ∥ diarize → meeting-merge) and submits them to **this same Executor**.

## Guarantees

The Executor never:

- owns ASR / context / fix / diarize / merge domain logic (implementations do)
- pretends `whisper` works before it exists; `meeting-merge` is a stub until alignment lands
- runs a second path beside the Job graph
- replaces `vd-srv` (queue) — this is the shared run engine; the queue submits Jobs

Full schema, CLI flags, exit codes: [cli.md](cli.md).
