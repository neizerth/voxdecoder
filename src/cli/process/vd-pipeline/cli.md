# vd-pipeline CLI

Universal **Job Executor**: DAG of capabilities with named artifacts. One Executor for CLI, `vd-meeting`, `vd-srv`, and MCP.

**Status: implemented** (Executor + linear Jobs; DAG fields and scheduling evolving).

Product notes: [README.md](README.md). Process overview: [../README.md](../README.md).

---

## Architecture

```text
CLI flags / Job file / vd-meeting / MCP / vd-srv
                    ↓
                   Job
                    ↓
                Executor
                    ↓
         Capabilities → implementations
```

There is **no** separate “standard mode” runtime.  
`vd-pipeline -i meeting.ogg` only **builds** a default linear Job, then runs the Executor.  
[`vd-meeting`](../vd-meeting/) only **builds** a meeting DAG Job and submits it to the **same** Executor.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-pipeline run` | Build or load a Job, then execute it |
| `vd-pipeline config` | Executor / CLI defaults |

Shorthand: `vd-pipeline -i FILE` ≡ `vd-pipeline run -i FILE`.

---

## CLI → Job

```bash
vd-pipeline run -i meeting.ogg
vd-pipeline run -i meeting.ogg --asr gigaam -m v3_e2e_ctc
vd-pipeline run -i meeting.ogg --device metal
vd-pipeline run -i meeting.ogg --docs ./docs
vd-pipeline run -i meeting.ogg --progress=json
vd-pipeline run -i meeting.ogg --dry-run --json
vd-pipeline run -i meeting.ogg --report report.json
vd-pipeline run job.yaml --report-dir ./run-out
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--input` | `-i` | — | Audio/video → Job `input.audio` (required unless a job file is given) |
| `--asr` | — | `gigaam` | Transcribe engine → `steps[transcribe].options.engine` |
| `--model` | `-m` | — | → `steps[transcribe].options.model` |
| `--device` | — | — | → `steps[transcribe].options.device` (`cpu` \| `metal` \| `cuda` \| `auto`; what the ASR binary accepts) |
| `--flash` | — | off | → `steps[transcribe].options.flash` (CUDA / non-mac `vd-gigaam` only) |
| `--docs` | — | `.` | Docs root → Job `context.docs` for always-on `prepare-context` (`vd-assets`) |
| `--output-dir` | `-d` | — | → Job `output.dir` |
| `--working-dir` | — | cwd | → Job `working_dir` (relative paths resolve here) |
| `--dry-run` | — | — | Print resolved Job and exit (no execution) |
| `--json` | — | — | With `--dry-run`: Job document on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` \| `json` (UI only; no timings) |
| `--quiet` | `-q` | — | Disable progress |
| `--continue-on-error` | — | off | Keep going after a failed step |
| `--overwrite` | — | — | Default for steps that support overwrite |
| `--report` | — | — | Write `ExecutionReport` JSON to this path |
| `--report-dir` | — | — | Write `report.json` + `resolved-job.json` into this directory |
| `--max-parallel` | — | config / `1` | Cap concurrent ready steps |

`--report` and `--report-dir` are mutually exclusive and require a real run (not `--dry-run`).

Job file vs CLI shorthand: pass a `.yaml` / `.yml` / `.json` (or `-f` / `--file`). Do not mix a job file with `-i` (exit 2).

### `--asr`

| Value | Behavior |
|-------|----------|
| `gigaam` | `use: transcribe` + `options.engine: gigaam` |
| `whisper` | same with `engine: whisper` — **reserved**; exit **2** until available |

---

## Job schema

Single format for files, `--dry-run --json`, builders (`vd-meeting`), and MCP.

`steps` is a **workflow tree**: each entry is a capability leaf (`use: …`) or a control node (`sequence` / `parallel`). A flat list of leaves is an implicit sequence (compat). See [WORKFLOW.md](WORKFLOW.md).

```yaml
version: 1
name: meeting cleanup          # optional job label

working_dir: .
max_parallel: 2                # concurrent parallel-branch fan-out

input:
  audio: meeting.ogg

context:
  docs: ./docs

output:
  dir: ./out

continue_on_error: false

steps:
  - use: transcribe
    id: transcript
    produces: [transcript]
    options:
      engine: gigaam
      model: v3_e2e_ctc

  - parallel:
      - use: fix-casing
        consumes: [transcript]
        id: cased
      - use: diarize
        id: timeline

  - sequence:
      - use: fix-asr
        input: cased
      - use: fix-terms
```

JSON is the same tree.

### Top-level fields

| Field | Required | Description |
|-------|----------|-------------|
| `version` | ✅ | Schema version (`1`) |
| `name` | — | Optional job label |
| `working_dir` | — | Base for relative paths |
| `input` | — | Job-level inputs (`audio`, …) |
| `context` | — | Shared context (`docs`, `assets`, …) |
| `output` | — | Output policy (`dir`, …) |
| `continue_on_error` | — | Same as CLI flag |
| `max_parallel` | — | Max concurrent ready steps |
| `resources` | — | Caps per resource group (`gpu` / `cpu` / `io`, …) |
| `steps` | ✅ | DAG nodes (list order is declaration order; edges from `inputs` / `depends`) |

### Step object

| Field | Required | Description |
|-------|----------|-------------|
| `use` | ✅ | Capability (see below) |
| `id` | — | Registers **primary** output as a named artifact |
| `name` | — | Human label only. **Omit when unset** — never for wiring |
| `inputs` | — | List of artifact ids or filesystem paths |
| `input` | — | Sugar for a single-entry `inputs` |
| `outputs` | — | Map of extra artifact name → path (implementation may fill paths) |
| `output` | — | Sugar / explicit path for the primary output |
| `produces` | — | Artifact names published (else `id` / `outputs`) |
| `consumes` | — | Artifact names required (else `inputs` / linear sugar) |
| `depends` | — | Extra step `id`s that must finish first (ordering without data) |
| `skip` | — | `true` → skip |
| `resource` | — | Resource group for this step (`gpu` / `cpu` / `io`, …) |
| `options` | — | Implementation-specific knobs only |

Omit `inputs` / `input` on a linear chain → previous step’s primary output (compat).

### `id` vs `name`

```yaml
- use: transcribe
  id: transcript
  name: Interview transcript

- use: fix-casing
  inputs:
    - transcript          # resolves via id, never via name
```

### DAG edges

```text
inputs: [a, b]   →  must wait until artifacts a and b exist
depends: [step-x] → must wait until step with id step-x completes
```

Independent ready steps may run together up to `max_parallel` and free resource slots.

### `options`

```yaml
- use: transcribe
  options:
    engine: gigaam
    model: v3_e2e_ctc
```

| Rule | Detail |
|------|--------|
| Unknown `options` key for the implementation | exit 2 |
| `engine: whisper` before implementation | exit 2 |
| Reserved capability before implementation | exit 2 |

### Capabilities → implementations

| `use` | Bound binary (detail) | Spec |
|-------|----------------------|------|
| `preprocess` | `vd-preprocess` | [README](../vd-preprocess/README.md) |
| `transcribe` + `engine: gigaam` | `vd-gigaam` | [cli](../../transcribe/vd-gigaam/cli.md) |
| `transcribe` + `engine: whisper` | `vd-whisper` | reserved |
| `prepare-context` | `vd-assets` | [cli](../vd-assets/cli.md) |
| `fix-casing` | `vd-fix-casing` | [cli](../../fix/vd-fix-casing/cli.md) |
| `fix-asr` | `vd-fix-asr` | [cli](../../fix/vd-fix-asr/cli.md) |
| `fix-terms` | `vd-fix-terms` | [cli](../../fix/vd-fix-terms/cli.md) |
| `diarize` | `vd-diarize` | [README](../vd-diarize/README.md) |
| `meeting-merge` | merge stub (in-process) | writes `meeting.json`; real merge later |
| `postprocess` | `vd-postprocess` | [README](../vd-postprocess/README.md) |

### Default Job shape (CLI)

Target shape once `vd-preprocess` lands — preprocess first (trim silence / normalize for ASR):

```yaml
version: 1
working_dir: .
input:
  audio: < -i >
context:
  docs: < --docs >          # only if --docs set
output:
  dir: < -d >               # only if -d set
steps:
  - use: preprocess
    id: prepared
    input: audio
    options:
      provider: ffmpeg
      filters:
        - type: trim-silence
        - type: normalize
  - use: transcribe
    input: prepared
  # … prepare-context / fix-* as today
```

Until `preprocess` is bound, the live default Job is:

```yaml
steps:
  - use: transcribe
    id: transcript
    options:
      engine: < --asr >
      model: < -m >
      device: < --device >
      flash: < --flash >           # if set
  - use: prepare-context          # always; docs from --docs or `.`
  - use: fix-casing
    input: transcript
  - use: fix-asr
  - use: fix-terms
```

`context.docs` defaults to `.` when `--docs` is omitted. If that root has no text/Office/PDF sources, the binder writes an empty `.voxdecoder/` and continues (fix-* still run).

---

## Named artifacts

```yaml
steps:
  - use: transcribe
    id: transcript

  - use: prepare-context
    id: assets
    outputs:
      terms: .voxdecoder/terms.yml
      md: .voxdecoder/md

  - use: fix-casing
    inputs: [transcript]
    id: cleaned

  - use: fix-asr
    inputs: [cleaned]
```

- `id` → primary artifact in the map.
- `outputs` → additional names registered when produced.
- Paths are resolved under `working_dir`; consumers use **names**, not hardcoded paths.
- Cycles in `inputs` / `depends` → exit 2.

---

## Behavior

1. Builder (CLI / file / `vd-meeting` / MCP) → Job.
2. Resolve `working_dir`, normalize `input` → `inputs`, validate artifacts + DAG (no cycles).
3. Gate reserved engines / capabilities.
4. `--dry-run` → print Job and exit 0.
5. Executor: schedule ready steps (parallel within limits); emit progress; stop unless `continue_on_error`.
6. Exit 0 or failing step’s code.

---

## `--dry-run`

Text lists steps with `id` / `inputs` / engines.  
`--dry-run --json`: Job document (same schema).

---

## Progress / status

Always know **which step**, **index/total** (or running/total for DAG), and **how far** the current capability has gotten.

[`vd-progress`](../../../crates/vd-progress/): `start` → `phase`* → `done` | `error` on stderr.

| Field | Meaning |
|-------|---------|
| `step` | Capability (`use`) |
| `id` | Artifact id when set — **omit when unset** |
| `name` | Human label — **omit when unset** |
| `path` | Filesystem path |

Statuses: `pending` \| `running` \| `done` \| `skipped` \| `failed`.

Progress is **UI only** — no `duration_ms`. Timings live in the Execution Report.

---

## Execution report

Durable profiling / audit JSON, separate from `vd-progress`.

```bash
vd-pipeline run job.yaml --report report.json
vd-pipeline run job.yaml --report-dir ./out
# → ./out/report.json
# → ./out/resolved-job.json
```

Shape (MVP):

```json
{
  "version": 1,
  "job": "meeting",
  "status": "ok",
  "started_at": "2026-08-02T10:00:00.000Z",
  "finished_at": "2026-08-02T10:01:12.000Z",
  "duration_ms": 72153,
  "steps": [
    {
      "id": "transcript",
      "capability": "transcribe",
      "status": "ok",
      "started_at": "…",
      "finished_at": "…",
      "duration_ms": 48132,
      "backend": "gigaam",
      "model": "v3_e2e_ctc",
      "inputs": [{ "path": "…", "bytes": 1234 }],
      "outputs": [{ "path": "…", "bytes": 567 }]
    }
  ]
}
```

`phases` is reserved (empty until child tools feed phase telemetry). On step failure the report is still written when `--report` / `--report-dir` is set.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success or dry-run |
| 1 | Step failed / Executor I/O |
| 2 | Bad Job / unknown capability / cycle / reserved engine or capability / CLI+file mix |
| 3 | Missing input / unreadable Job file / missing `-i` |
| other | Propagated from implementation when ≥ 4 |

---

## Config

```bash
vd-pipeline config list
vd-pipeline config get progress
vd-pipeline config set progress json
vd-pipeline config set asr gigaam
vd-pipeline config set max_parallel 2
vd-pipeline config path
```

| Key | Default | Description |
|-----|---------|-------------|
| `progress` | `text` | Progress mode |
| `asr` | `gigaam` | Default transcribe engine for CLI → Job |
| `continue_on_error` | `off` | Default stop-on-error |
| `max_parallel` | `1` | Default concurrency for ready steps |

`$VD_PIPELINE_CONFIG` or platform config dir.

Priority: CLI > Job top-level > config > default.

---

## Public contract note

**Job + Executor** are the product contract.  
CLI and `vd-meeting` are Job builders. MCP / `vd-srv` submit the same Job JSON. How a capability is implemented is an internal detail.
