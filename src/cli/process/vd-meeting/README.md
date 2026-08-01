# vd-meeting — Meeting Planner

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI / Meeting document: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [vd-pipeline](../vd-pipeline/) · [vd-diarize](../vd-diarize/) · [../../fix/README.md](../../fix/README.md).  
Shared crates (planned): [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/), [`vd-pipeline`](../vd-pipeline/).  
Rust gates: [RUST.md](RUST.md).

**Status: planned.** Workspace member to be `src/cli/process/vd-meeting`.

## Core rule

```text
vd-meeting is a Meeting Planner.

It validates the Meeting Model, normalizes inputs, plans a Job DAG
(transcript branches / diarize / meeting-merge), wires artifacts,
and submits the Job to the common Executor.

It does not execute steps itself.
```

> **vd-meeting plans meetings into Jobs.**  
> It constructs a DAG of standard VoxDecoder capabilities (`transcribe`, `fix-*`, `diarize`, `meeting-merge`) and submits that Job to the Executor used by `vd-pipeline`, `vd-srv`, and the future MCP server.

User-facing nickname “Job Builder” is fine — planning’s last step is emitting a Job. Internally: **MeetingPlanner** ([STRUCTURE.md](STRUCTURE.md)).

## Model hierarchy

```text
MeetingRequest          # meeting domain only
        ↓
BuildOptions            # executor + transcribe defaults (separate)
        ↓
MeetingPlanner
        ↓
Job
        ↓
Executor
        ↓
Artifacts
        ↓
Meeting Artifact        # ArtifactType::Meeting
```

| Layer | Knows | Does not know |
|-------|-------|----------------|
| CLI / MCP | document, flags | Executor internals |
| MeetingPlanner | Meeting Model, roles, Job shape | GigaAM / pyannote |
| Executor | Job DAG | what a meeting is |
| Capabilities | their domain | MeetingRequest |

## Architecture

```text
inputs + Meeting Model (+ BuildOptions)
              │
              ▼
         vd-meeting          ← MeetingPlanner
              │
              ▼
            Job (DAG)
              │
              ▼
         Executor            ← shared (vd-pipeline / vd-srv / MCP)
              │
    ┌─────────┼─────────┐
    ▼         ▼         ▼
 transcript* diarize  meeting-merge capability
    │         │         │
    └─────────┴─────────┘
              │
     Meeting Artifact (ArtifactType::Meeting)
              │
   meeting.json / .srt / .txt / .md / …
```

| Layer | Role |
|-------|------|
| **`vd-meeting`** | Plan MeetingRequest → Job |
| **[`vd-pipeline` Executor](../vd-pipeline/)** | Schedule / parallel / bind capabilities |
| **transcript branch** | `transcribe → fix-*` (later: translate, summarize, …) per recording that needs text |
| **`diarize`** | [`vd-diarize`](../vd-diarize/) — speaker timeline only |
| **`meeting-merge` capability** | Align, match speakers, emit Meeting Artifact |

## Why not an orchestrator

There is **one** executor. CLI, server, MCP, and `vd-meeting` only differ in **how they obtain a Job**. Meeting-specific knowledge lives in the planner — not in a second runtime.

Each audio source that needs text becomes a **transcript branch** (not a “cleanup” branch — the chain will grow):

```text
transcribe → fix-casing → fix-asr → fix-terms
# later: translate · summarize · llm-review · …
```

Branches are sibling subgraphs. The Executor runs them in parallel when independent.

`diarize` is a **separate branch** when the planner decides it is needed — never inside a transcript branch.

---

## Inputs

There are **no modes**. There are **sources**. The Job is **derived from available inputs**.

### Input roles

```yaml
inputs:
  - role: merged
    path: meeting.wav

  - role: participant
    participant: alice
    path: alice.wav

  - role: participant
    participant: bob
    path: bob.wav

  - role: context
    path: ./docs          # README, wiki, pdf, jira, slides, … — vd-assets decides
```

| Role | Meaning |
|------|---------|
| `merged` | Room / mixed recording |
| `participant` | Per-person track (optional link to known participant) |
| `context` | Project materials for `prepare-context` — not limited to “docs” |

Later roles stay the same idea (`reference`, `screen-recording`, `zoom-audio`, …).

Examples (all valid; no special mode):

```yaml
inputs:
  - role: participant
    path: speaker1.wav
  - role: participant
    path: speaker2.wav
```

```yaml
inputs:
  - role: merged
    path: meeting.wav
```

```yaml
inputs:
  - role: merged
    path: meeting.wav
  - role: participant
    participant: alice
    path: alice.wav
  - role: context
    path: ./docs
```

---

## Meeting Model

Describes the **meeting itself**. Same object for `vd-meeting`, MCP, and the Job envelope.

```yaml
meeting:
  participants:
    known:
      - name: Alice
        constraints:
          gender: female
      - name: Bob
        constraints:
          gender: male
      - id: charlie
        name: Charlie
        optional: false

    expected:                 # count of unnamed / not-in-known speakers
      min: 0
      max: 2

    constraints:
      min: 2
      max: 5
      genders:
        male: { min: 1, max: 3 }
        female: { min: 1, max: 2 }

  diarization:
    enabled: auto

  alignment:
    mode: longest
    # tolerance_ms: 500
    # allow_clock_drift: false
```

Job knobs (`overwrite`, `max_parallel`, ASR engine) are **not** part of Meeting Model — they live in BuildOptions / CLI ([STRUCTURE.md](STRUCTURE.md)).

### Participants

| Field | Role |
|-------|------|
| `known` | Named / identifiable people |
| `expected` | Bounds for speakers not listed in `known` |
| `constraints` | Global `min` / `max` / `genders` |
| `optional` on a known person | May be absent from the recording |

Minimal forms:

```yaml
meeting:
  participants:
    known:
      - name: Alice
      - name: Bob
```

```yaml
meeting:
  participants:
    constraints:
      min: 4
      max: 4
```

`id` may be omitted — planner generates one. Per-person hints use typed `constraints:` (`gender`, later `age`, `language`, …) — not a free-form map.

### Participant constraints

| Scenario | Example |
|----------|---------|
| Everyone known | `known: […]`, tight `constraints.min/max` |
| Partial roster | some `known` + `expected.min/max` |
| Count only | `constraints.min/max` |
| Gender mix | `constraints.genders` |
| Soft presence | `optional: true` |

Matching `S* →` labels is the **`meeting-merge` capability**’s job — never `vd-diarize`.

### Diarization policy

```yaml
meeting:
  diarization:
    enabled: auto    # auto | true | false
```

| Value | Meaning |
|-------|---------|
| `auto` | Add `diarize` when a merged-like source exists and diarization is useful |
| `true` | Prefer / require `diarize` when possible |
| `false` | Never attach `diarize` |

### Alignment

```yaml
meeting:
  alignment:
    mode: start
    tolerance_ms: 500
    allow_clock_drift: false
```

| `mode` | Behavior |
|--------|----------|
| `longest` | Longest recording is the reference |
| `start` | Shared start; late joiners get leading silence |
| `end` | Shared end; early leavers get trailing silence |

Copied onto the **`meeting-merge` capability** options by the planner.

---

## Planner algorithm

```text
collect inputs
      ↓
normalize Meeting Model
      ↓
build transcript branches
      ↓
build diarization branch
      ↓
build meeting-merge capability
      ↓
resolve artifacts
      ↓
submit Job
```

1. **Collect inputs** — declared roles, paths, participant links (no filesystem crawl).
2. **Normalize Meeting Model** — ids, defaults, constraint consistency → `ResolvedMeeting`.
3. **Determine branches** — what the input set + policy allow.
4. **Build transcript branches** — one `transcribe → …` chain per source that needs text.
5. **Build diarization branch** — if merged-like source + `diarization.enabled` allows.
6. **Build `meeting-merge` capability** — transcripts (+ timeline) + Meeting Model options.
7. **Resolve / wire artifacts** — named ids for Executor.
8. **Submit Job** (or return Job for dry-run / MCP).

### Example graph

```text
alice.wav  ──► [transcript branch] ──► alice.transcript ──┐
bob.wav    ──► [transcript branch] ──► bob.transcript   ──┼──► meeting-merge capability ──► Meeting Artifact
meeting.wav ─► diarize ──────────────► SpeakerTimeline ────┘
```

If only tracks exist, or `diarization.enabled: false`, omit diarize.  
If only merged exists, transcript (+ optional diarize) still feed merge.

---

## Meeting Artifact

Canonical result of **`meeting-merge`**: `ArtifactType::Meeting` (participants, segments, timeline, links).

Exports of the same object:

```text
meeting.json                 # canonical
meeting.srt | .txt | .md
participants/alice.txt …
meeting.diarization.json     # when diarize ran
meeting.timeline.json
meeting.metadata.json
```

---

## Boundaries

| Tool | Owns |
|------|------|
| [`vd-diarize`](../vd-diarize/) / `use: diarize` | Who spoke when (`S*`) — no Meeting Model |
| [`vd-pipeline`](../vd-pipeline/) Executor | Run any Job DAG |
| **`vd-meeting`** | MeetingRequest + BuildOptions → Job; submit only |

## Guarantees

`vd-meeting` never:

- runs steps or embeds ASR / diarize / fix engines
- puts Job knobs (`overwrite`, `max_parallel`, …) inside Meeting Model / MeetingRequest
- invents participant names without model data or a successful merge match
- treats exports as separate truth from `ArtifactType::Meeting`
- replaces the shared Executor
- hard-codes “modes” — the Job follows from **available inputs** + Meeting Model
