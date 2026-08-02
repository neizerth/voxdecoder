# vd-meeting CLI

**Meeting Planner**: inputs + Meeting Model → Job DAG → shared Executor.

**Status: implemented.**

Product: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md). Process: [../README.md](../README.md). Executor: [../vd-pipeline/cli.md](../vd-pipeline/cli.md).

---

## Architecture

```text
CLI flags
Meeting document
MCP JSON

        ↓

  MeetingRequest          # meeting domain only
  BuildOptions            # executor + transcribe defaults (separate)

        ↓

  MeetingPlanner::plan    ← this crate

        ↓

      Job (DAG)

        ↓

   Executor               ← vd-pipeline (shared)
```

There is **no** meeting-specific runtime.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-meeting run` | Plan Job from inputs + Meeting Model, then submit (or dry-run) |
| `vd-meeting plan` | Plan Job and print it (alias of `run --dry-run`) |
| `vd-meeting config` | Planner / BuildOptions defaults |

Shorthand: `vd-meeting …` without subcommand inserts `run`.

---

## CLI → MeetingRequest + BuildOptions

```bash
vd-meeting run \
  --input role=room,path=meeting.wav,purposes=timeline \
  --input role=participant,participant=alice,path=alice.wav \
  --input role=context,path=./docs \
  --meeting meeting.yaml \
  --dry-run --json

vd-meeting run meeting.yaml
vd-meeting run meeting.yaml --context ./docs
```

### MeetingRequest (domain)

| Argument | Short | Description |
|----------|-------|-------------|
| `--input` | — | Repeatable: `role=…,path=…[,participant=…][,purposes=transcript\|timeline]` |
| `--meeting` / positional | `-f` | Document with `inputs` + `meeting` |
| `--context` | — | Sugar: add `role: context` input |
| `--output-dir` | `-d` | → MeetingOutput / merge output dir |
| `--working-dir` | — | Relative path base |

### BuildOptions (not part of Meeting Model)

| Argument | Description |
|----------|-------------|
| `--asr` / `-m` | Transcribe defaults → Job `transcribe` options |
| `--overwrite` | Transcribe / fix / merge overwrite |
| `--max-parallel` | → Job `max_parallel` |
| `--continue-on-error` | → Job flag |
| `--progress` / `-q` | Progress UX |

Do not invent mode flags. Presence of inputs decides the graph.

---

## Meeting document

```yaml
version: 1
working_dir: .

inputs:
  - role: room
    path: meeting.wav
    # default with tracks: purposes: [timeline]
  - role: participant
    participant: alice
    path: alice.wav
  - role: participant
    participant: bob
    path: bob.wav
  - role: context
    path: ./docs

meeting:
  participants:
    known:
      - id: alice
        name: Alice
        constraints:
          gender: female
      - name: Bob
        constraints:
          gender: male
    expected:
      min: 0
      max: 1
    constraints:
      min: 2
      max: 4
      genders:
        male: { min: 1, max: 2 }
        female: { min: 1, max: 2 }

  diarization:
    enabled: auto

  alignment:
    mode: longest

output:
  dir: ./out
```

ASR / overwrite / parallelism are **CLI or config BuildOptions**, not fields under `meeting:`.

### Top-level

| Field | Required | Description |
|-------|----------|-------------|
| `version` | ✅ | `1` |
| `working_dir` | — | Path base |
| `inputs` | ✅ | Sources with **roles** |
| `meeting` | — | Meeting Model |
| `output` | — | Output policy |

### Input object

| Field | Required | Description |
|-------|----------|-------------|
| `role` | ✅ | `room` (alias `merged`) \| `participant` \| `context` |
| `path` | ✅ | Filesystem path |
| `participant` | — | Link to known id/name |
| `purposes` | — | `[transcript]` and/or `[timeline]`; empty → planner defaults |

CLI: `--input role=room,path=meeting.wav,purposes=timeline`  
(`purposes` values separated by `\|` when both needed).

Defaults: participant → transcript; room + tracks → timeline only; room alone → transcript (+ timeline if diarization auto/true).

### Meeting Model

| Block | Description |
|-------|-------------|
| `participants.known` | People; typed `constraints`; `optional` |
| `participants.expected` | Bounds for speakers not in `known` |
| `participants.constraints` | Global `min` / `max` / `genders` |
| `diarization.enabled` | `auto` \| `true` \| `false` |
| `alignment` | Nested `mode` (+ future knobs) |

---

## Planner → Job (sketch)

```yaml
version: 1
working_dir: .
max_parallel: 2          # from BuildOptions
steps:
  - use: prepare-context
    id: assets
    # from role: context

  - use: transcribe
    id: alice.transcript
    input: alice.wav
    options: { engine: gigaam }   # from BuildOptions.transcribe

  - use: fix-casing
    inputs: [alice.transcript]
    id: alice.cased
  - use: fix-asr
    inputs: [alice.cased]
    id: alice.asr
  - use: fix-terms
    inputs: [alice.asr]
    id: alice.text

  - use: diarize
    id: timeline
    input: meeting.wav

  - use: meeting-merge
    id: meeting
    inputs: [alice.text, bob.text, timeline]
    options:
      alignment: { mode: longest }
      participants: { … }
```

Each participant (or room-with-transcript) path is a **transcript branch**. A room mix with only `purpose: timeline` does **not** get ASR.

---

## Behavior

1. Parse → `MeetingRequest` + `BuildOptions`.
2. Validate / normalize Meeting Model.
3. `--dry-run` → print Job → exit 0.
4. `MeetingPlanner::plan` → Job.
5. Submit to Executor.
6. Exit 0 or validation / Executor code.

---

## `--dry-run`

`--dry-run --json`: **Job** document (Executor schema).

---

## Progress

Planner phases: `collecting`, `normalizing`, `planning`, `submitting`.  
After submit: Executor progress from `vd-pipeline`.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success or dry-run |
| 1 | Executor / I/O |
| 2 | Bad document / unknown role / invalid constraints |
| 3 | Missing inputs / unreadable paths |

---

## Config

```bash
vd-meeting config list
vd-meeting config get diarization.enabled
vd-meeting config set alignment.mode longest
vd-meeting config path
```

| Key | Default | Layer |
|-----|---------|--------|
| `diarization.enabled` | `auto` | Meeting Model default |
| `alignment.mode` | `longest` | Meeting Model default |
| `asr` | (pipeline) | BuildOptions.transcribe |
| `max_parallel` | (pipeline) | BuildOptions.executor |
| `progress` | `text` | UX |

`$VD_MEETING_CONFIG` or platform config dir.

Priority: CLI > meeting document > config > default.

---

## Public contract note

**MeetingRequest + BuildOptions → Job** is the contract.  
`ArtifactType::Meeting` is the canonical merge result. Execution always goes through the shared Executor.
