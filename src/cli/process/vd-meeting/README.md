# vd-meeting — Meeting Planner

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI / Meeting document: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [vd-pipeline](../vd-pipeline/) · [vd-diarize](../vd-diarize/) · [../../fix/README.md](../../fix/README.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/), [`vd-pipeline`](../vd-pipeline/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented.** Workspace member: `src/cli/process/vd-meeting`. Track alignment (`alignment.mode: longest`) pads shorter recordings with leading silence in preprocess before ASR/diarize. `meeting-merge` still uses a stub binder in `vd-pipeline` for the final `meeting.json` (acoustic force-align remains future work).

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
transcribe → fix-casing → fix-asr → fix-disfluency → fix-terms → fix-layout
# later: translate · summarize · llm-review · …
# after meeting-merge (when diarized): fix-overlap
```

Branches are sibling subgraphs. The Executor runs them in parallel when independent.

`diarize` is a **separate branch** when the planner decides it is needed — never inside a transcript branch.

---

## Inputs

There are **no modes**. There are **sources**. The Job is **derived from available inputs**.

`role` describes **what the file is** (provenance).  
`purposes` describes **why it is used** (transcript vs timeline). They are orthogonal.

### Input purposes

| Purpose | Meaning |
|---------|---------|
| `transcript` | Build a transcript branch (`transcribe` → `fix-*`) |
| `timeline` | Feed `diarize` → `SpeakerTimeline` |

Omit `purposes` to use defaults:

| Role | Situation | Default purposes |
|------|-----------|------------------|
| `participant` | any | `[transcript]` |
| `room` | with participant tracks | `[timeline]` only (mix for diarize **or** merge alignment reference — **no** room ASR by default) |
| `room` | alone, diarization auto/true | `[transcript, timeline]` |
| `room` | alone, diarization false | `[transcript]` |
| `context` | any | (none) |

With participant tracks, the room mix can still feed **`meeting-merge` without diarization**: set `meeting.diarization.enabled: false` and `meeting.alignment.reference: mix` (or `auto`). The planner passes prepared room audio into merge as a timing reference; track transcripts supply text and speaker identity. `alignment.reference: none` ignores the mix (tracks only). By default (`alignment.mode: longest`) shorter tracks get leading silence in preprocess so ASR clocks share the longest timeline; acoustic force-align remains future work.

### Input roles

```yaml
inputs:
  - role: room                 # alias: merged
    path: meeting.wav
    purposes: [timeline]       # optional; default when tracks exist

  - role: participant
    participant: alice
    path: alice.wav

  - role: participant
    participant: bob
    path: bob.wav

  - role: context
    path: ./docs
```

| Role | Meaning |
|------|---------|
| `room` | Multi-speaker / room mix (`merged` still accepted as alias) |
| `participant` | Per-person track (optional link to known participant) |
| `context` | Project materials for `prepare-context` |

Examples:

```yaml
# Tracks only — no diarize
inputs:
  - role: participant
    path: speaker1.wav
  - role: participant
    path: speaker2.wav
```

```yaml
# Room alone — transcript (+ timeline when diarization auto/true)
inputs:
  - role: room
    path: meeting.wav
```

```yaml
# Primary multi-track case: room = timeline only
inputs:
  - role: room
    path: meeting.wav
  - role: participant
    participant: alice
    path: alice.wav
  - role: participant
    participant: bob
    path: bob.wav
  - role: context
    path: ./docs
```

> **Room is not “always transcribe.”** With isolated tracks it is usually a **timeline source** only. Override with `purposes: [transcript, timeline]` if you also want ASR on the mix.

Future direction: property-based sources (`kind`, `channels: mixed|isolated`, …) without changing the purpose model.
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
    # reference: auto          # auto | mix | timeline | none
    #   auto: timeline if diarize ran, else mix if room present, else none
    #   mix: room audio into meeting-merge (no diarize required)
    #   timeline: require diarize timeline
    #   none: tracks only (ignore mix)
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
| `auto` | Add `diarize` when some input has purpose `timeline` |
| `true` | Prefer / require `diarize` when a timeline source exists |
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
| `longest` (default) | Probe all tracks (+ room); pad shorter ones with **leading** silence so every file matches the longest duration (assumes late joiners / shared end). Applied in preprocess via `pad-start` before ASR/diarize. |
| `start` | Same leading pad as `longest`. |
| `end` | Pad shorter tracks with **trailing** silence (`pad-end`) to match the longest. |

The planner injects `pad-start` / `pad-end` into each branch’s preprocess chain (before `normalize`, provider `ffmpeg`). **`trim-silence` is omitted** for meeting Jobs — uniform TimeMap cannot represent silenceremove (ADR 0016). Mode is also copied onto **`meeting-merge`** options for the artifact metadata.
---

## Planner algorithm

```text
collect inputs
      ↓
normalize Meeting Model
      ↓
resolve purposes (explicit or defaults)
      ↓
determine artifact requirements
      ↓
determine branches
      ↓
reuse sources when possible
      ↓
build transcript / diarize / meeting-merge
      ↓
submit Job
```

1. **Collect inputs** — roles, paths, participant links, optional purposes.
2. **Normalize Meeting Model** — ids, defaults, constraint consistency.
3. **Resolve purposes** — fill defaults (room+tracks → timeline-only, …).
4. **Determine artifact requirements** — which transcripts + whether `SpeakerTimeline`.
5. **Determine branches** — map requirements → sources (no wasted room ASR by default).
6. **Build transcript branches** — one chain per source with purpose `transcript`.
7. **Build diarization branch** — if policy allows and a `timeline` purpose source exists.
8. **Build `meeting-merge`** — transcript artifacts (+ timeline) + Meeting Model options.
9. **Submit Job** (or return Job for dry-run / MCP).

### Example graph (room + tracks)

```text
alice.wav  ──► [transcript branch] ──► alice.text ──┐
bob.wav    ──► [transcript branch] ──► bob.text   ──┼──► meeting-merge ──► Meeting Artifact
meeting.wav ─► diarize ──────────────► timeline ────┘
                 (purpose: timeline only — no room.transcript)
```

If only tracks exist, or `diarization.enabled: false`, omit diarize.  
If only room exists, transcript (+ optional diarize) still feed merge.
---

## Meeting Artifact

Canonical result of **`meeting-merge`**:

- `meeting.json` — machine artifact (participants roster, turns, timeline, alignment)
- `meeting.md` — human transcript (`[Speaker]\ntext` blocks); prefer this for reading / sharing

Participants in the JSON are **display names** for each text track (from `participants.known[].name` when set), not duplicated id+name pairs.

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
