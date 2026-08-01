# vd-pipeline CLI

Execute a **Job**: ordered capabilities with named artifacts, one Executor for CLI and (later) MCP.

**Status: implemented.**

Product notes: [README.md](README.md). Process overview: [../README.md](../README.md).

---

## Architecture

```text
CLI flags
Job file
MCP JSON

        ↓

       Job

        ↓

    Executor

        ↓

  Capabilities  →  implementations (lib / CLI / process)
```

There is **no** separate “standard mode” runtime.  
`vd-pipeline -i meeting.ogg` only **builds** the default Job, then runs the Executor.

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
vd-pipeline run -i meeting.ogg --asr gigaam -m v2_rnnt
vd-pipeline run -i meeting.ogg --docs ./docs
vd-pipeline run -i meeting.ogg --progress=json
vd-pipeline run -i meeting.ogg --dry-run --json
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--input` | `-i` | — | Audio/video → Job `input.audio` (required unless a job file is given) |
| `--asr` | — | `gigaam` | Transcribe engine → `steps[transcribe].options.engine` |
| `--model` | `-m` | — | → `steps[transcribe].options.model` |
| `--docs` | — | — | Docs root → Job `context.docs`; adds a `prepare-context` step when set |
| `--output-dir` | `-d` | — | → Job `output.dir` |
| `--working-dir` | — | cwd | → Job `working_dir` (relative paths resolve here) |
| `--dry-run` | — | — | Print resolved Job and exit (no execution) |
| `--json` | — | — | With `--dry-run`: Job document on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` \| `json` |
| `--quiet` | `-q` | — | Disable progress |
| `--continue-on-error` | — | off | Keep going after a failed step |
| `--overwrite` | — | — | Default for steps that support overwrite |

Job file vs CLI shorthand: pass a `.yaml` / `.yml` / `.json` (or `-f` / `--file`). Do not mix a job file with `-i` (exit 2).

### `--asr`

| Value | Behavior |
|-------|----------|
| `gigaam` | `use: transcribe` + `options.engine: gigaam` |
| `whisper` | same with `engine: whisper` — **reserved**; Executor exits **2** (`asr_not_implemented`) until available |

---

## Job schema

Single format for files, `--dry-run --json`, and future MCP.

```yaml
version: 1
name: meeting cleanup          # optional job label

working_dir: .                 # optional; default cwd

input:
  audio: meeting.ogg

context:
  docs: ./docs                 # optional; drives prepare-context when present
  # assets: ./.voxdecoder      # optional explicit context dir

output:
  dir: ./out                   # optional

continue_on_error: false

steps:
  - use: transcribe
    id: transcript               # artifact id (wiring)
    name: Initial transcript     # optional human label
    options:
      engine: gigaam             # gigaam | whisper
      model: v2_rnnt
      device: cuda
      flash: true

  - use: prepare-context         # vd-assets → .voxdecoder
    options:
      ocr: false

  - use: fix-casing
    input: transcript            # artifact id from earlier step

  - use: fix-asr
    # input omitted → previous step’s primary output

  - use: fix-terms
```

JSON is the same tree.

### Top-level fields

| Field | Required | Description |
|-------|----------|-------------|
| `version` | ✅ | Schema version (`1`) |
| `name` | — | Optional job label (progress on `start` when set) |
| `working_dir` | — | Base for relative paths |
| `input` | — | Job inputs (`audio`, …) |
| `context` | — | Shared context (`docs`, `assets`, …) |
| `output` | — | Output policy (`dir`, …) |
| `continue_on_error` | — | Same as CLI flag |
| `steps` | ✅ | Ordered capabilities |

### Step object

| Field | Required | Description |
|-------|----------|-------------|
| `use` | ✅ | Capability: `transcribe` \| `prepare-context` \| `fix-casing` \| `fix-asr` \| `fix-terms` |
| `id` | — | Artifact id for wiring. Later steps may `input: <id>` |
| `name` | — | Optional human label (UI / logs / progress). **Omit when unset** — not used for wiring |
| `input` | — | Artifact `id` or filesystem path. Omit → previous step’s primary output |
| `output` | — | Optional explicit path (else implementation / Job `output.dir` defaults) |
| `skip` | — | `true` → skip (status `skipped`) |
| `options` | — | Implementation-specific knobs only |

Reserved for later (not in `options`): `when`, `depends_on`, `retry`, `timeout`, …

### `id` vs `name`

```yaml
- use: transcribe
  id: transcript              # machine id → input: transcript
  name: Interview transcript  # humans / progress only

- use: fix-casing
  input: transcript           # resolves via id, never via name
```

Do not put spaces or display copy in `id`.

### `options`

Maps to the bound implementation’s `run` long flags (without `--`), nested so they never collide with Job fields.

```yaml
- use: transcribe
  options:
    engine: gigaam
    model: v2_rnnt
    device: cuda
    flash: true

- use: prepare-context
  options:
    ocr: true
    force: true

- use: fix-asr
  options:
    language: ru
```

| Rule | Detail |
|------|--------|
| Unknown `options` key for the implementation | exit 2 |
| Booleans | `true` / `false` |
| Repeatable flags | YAML/JSON array |
| `engine: whisper` before implementation | exit 2 |

### Capabilities → implementations

| `use` | Bound binary (implementation detail) | Spec |
|-------|--------------------------------------|------|
| `transcribe` + `engine: gigaam` | `vd-gigaam` | [cli](../../transcribe/vd-gigaam/cli.md) |
| `transcribe` + `engine: whisper` | `vd-whisper` | TBD — reserved |
| `prepare-context` | `vd-assets` | [cli](../vd-assets/cli.md) |
| `fix-casing` | `vd-fix-casing` | [cli](../../fix/vd-fix-casing/cli.md) |
| `fix-asr` | `vd-fix-asr` | [cli](../../fix/vd-fix-asr/cli.md) |
| `fix-terms` | `vd-fix-terms` | [cli](../../fix/vd-fix-terms/cli.md) |

How a capability runs (subprocess vs in-process) is **outside** the Job contract.

### Default Job shape (what CLI builds)

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
  - use: transcribe
    id: transcript
    options:
      engine: < --asr >
      model: < -m >           # if set
  - use: prepare-context      # only if --docs set
  - use: fix-casing
    input: transcript
  - use: fix-asr
  - use: fix-terms
```

---

## Artifacts (named wiring)

```yaml
steps:
  - use: transcribe
    id: transcript

  - use: fix-casing
    input: transcript
    id: cleaned

  - use: fix-asr
    input: cleaned
```

- Linear execution for now (no DAG / `depends_on` yet).
- `id` registers the step’s primary output in an artifact map.
- `input: <id>` resolves via that map; a path-like string is a filesystem path.
- Omitting `input` → previous step’s primary output.

---

## Behavior

1. CLI flags → Job **or** load Job file / MCP payload.
2. Resolve `working_dir` and relative paths.
3. Resolve artifact `id`s → paths; reject unknown `input` ids (exit 2).
4. Gate reserved engines (`whisper`).
5. `--dry-run` → print Job (text or `--json`) and exit 0.
6. Executor runs steps in order; emit progress; stop unless `continue_on_error`.
7. Exit 0 or failing step’s code.

---

## `--dry-run`

Text:

```text
Job: (unnamed)
working_dir: .
input.audio: meeting.ogg
steps: 4
  1. transcribe        id=transcript  engine=gigaam  model=v2_rnnt
  2. fix-casing        input=transcript
  3. fix-asr           input=<prev>
  4. fix-terms         input=<prev>
```

`--dry-run --json`: the resolved Job document (same schema as the file).

---

## Progress / status

Always know **which step**, **index/total**, and **how far** the current capability has gotten.

[`vd-progress`](../../../crates/vd-progress/): `start` → `phase`* → `done` | `error` on stderr.

| Value | Description |
|-------|-------------|
| `text` | Status board on stderr (default) |
| `json` | NDJSON on stderr |
| `-q` | No orchestrator progress |

### Text

```text
job  meeting.ogg  engine=gigaam
[1/4] transcribe    running   transcribing  42%
[2/4] fix-casing    pending
[3/4] fix-asr       pending
[4/4] fix-terms     pending
```

Statuses: `pending` \| `running` \| `done` \| `skipped` \| `failed`.

### JSON

| Field | Meaning |
|-------|---------|
| `span` / `span_total` | Step index / count |
| `segment` / `segment_total` | Implementation span counters when present |
| `step` | Capability id (`transcribe`, `prepare-context`, …) — value of `use` |
| `id` | Optional artifact id when the step declares one. **Omit when unset** |
| `name` | Optional human label. **Omit when unset** |
| `path` | Filesystem path (input while running; primary output on `step_done`) |
| `phase` | Lifecycle / implementation phase |

`path` is always a file/dir path. Capability id is `step`. Wiring id is `id`. Display copy is `name`.

| `phase` | Meaning |
|---------|---------|
| `step_start` | Step begins |
| `{step}:{child_phase}` | Forwarded phase (e.g. `transcribe:transcribing`) |
| `step_done` | Succeeded (`path` = primary output) |
| `step_skipped` | Skipped |
| `step_failed` | Failed |

```json
{"event":"start","artifact_type":"job","input":"./meeting.ogg","model":"v2_rnnt"}
{"event":"phase","phase":"step_start","percent":0,"span":1,"span_total":4,"step":"transcribe","id":"transcript","path":"./meeting.ogg"}
{"event":"phase","phase":"transcribe:transcribing","percent":8,"span":1,"span_total":4,"step":"transcribe","id":"transcript","path":"./meeting.ogg"}
{"event":"phase","phase":"step_done","percent":25,"span":1,"span_total":4,"step":"transcribe","id":"transcript","path":"./meeting.txt"}
{"event":"phase","phase":"step_start","percent":25,"span":2,"span_total":4,"step":"fix-casing","path":"./meeting.txt"}
{"event":"phase","phase":"fix-casing:fixing","percent":30,"span":2,"span_total":4,"segment":1,"segment_total":20,"step":"fix-casing","name":"Polish casing","path":"./meeting.txt"}
{"event":"phase","phase":"step_done","percent":50,"span":2,"span_total":4,"step":"fix-casing","path":"./meeting.fixed.txt"}
{"event":"done","output":"./meeting.fixed.txt","path":"./meeting.fixed.txt","duration_sec":12.4}
{"event":"error","code":"asr_not_implemented","message":"whisper is reserved; not available yet"}
```

`span` / `span_total` = Job steps; `segment` / `segment_total` = implementation spans.

**Overall `percent`**: `(completed_steps + child_fraction) / total_steps`.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success or dry-run |
| 1 | Step failed with 1 / Executor I/O error |
| 2 | Bad Job / unknown capability or option / CLI+file mix / reserved engine |
| 3 | Missing input / unreadable Job file / missing `-i` when building default Job |
| other | Propagated from implementation when ≥ 4 |

---

## Config

```bash
vd-pipeline config list
vd-pipeline config get progress
vd-pipeline config set progress json
vd-pipeline config set asr gigaam
vd-pipeline config path
```

| Key | Default | Description |
|-----|---------|-------------|
| `progress` | `text` | Progress mode |
| `asr` | `gigaam` | Default transcribe engine for CLI → Job |
| `continue_on_error` | `off` | Default stop-on-error |

`$VD_PIPELINE_CONFIG` or platform config dir.

Priority: CLI > Job top-level > config > default.

---

## Public contract note

**Job + Executor** are the product contract.  
CLI is a Job builder. MCP submits the same Job JSON. How a capability is implemented is an internal detail.
