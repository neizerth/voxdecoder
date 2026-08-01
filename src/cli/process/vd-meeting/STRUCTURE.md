# vd-meeting — project layout

Rust crate: **Meeting Planner**. Validates / normalizes a meeting, plans a Job DAG, submits it to the shared [`vd-pipeline`](../vd-pipeline/) Executor. Does not run steps.

**Status: planned.** Workspace member to be `src/cli/process/vd-meeting`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md) · [../vd-pipeline/](../vd-pipeline/) · [../vd-diarize/](../vd-diarize/)

---

## Philosophy

```text
MeetingRequest
      ↓
MeetingPlanner          ← this crate (validate · normalize · plan graph · wire artifacts)
      ↓
     Job
      ↓
  Executor              ← vd-pipeline (shared)
      ↓
  Artifacts
      ↓
Meeting Artifact        ← ArtifactType::Meeting
```

Layers do not leak:

| Layer | Knows | Does not know |
|-------|-------|----------------|
| **CLI / MCP** | flags, meeting document | Executor internals |
| **MeetingPlanner** | Meeting Model, input roles, Job shape | GigaAM / pyannote / … |
| **Executor** | Job DAG, capabilities | what a “meeting” is |
| **Capabilities** | their domain | MeetingRequest |

User-facing shorthand “Job Builder” still fits (planner’s last duty is emitting a Job). Internally the center is **`planner/`**, not `build/`.

- **No private orchestrator** — one Executor for CLI / MCP / `vd-srv` / this planner.
- **No modes** — Job is derived from **available inputs** (roles) + Meeting Model.
- **`meeting-merge` is a capability**; planner only emits that step.
- **`diarize` is a separate branch** when policy + inputs allow it — never inside a transcript branch.
- Meeting knowledge lives here — not in `vd-diarize`.

Product intent: [README.md](README.md).

---

## Model hierarchy

```text
MeetingRequest          # what the user wants (meeting domain only)
        ↓
BuildOptions            # how to plan/run (executor + transcribe defaults) — separate
        ↓
MeetingPlanner
        ↓
Job                     # what the Executor runs
        ↓
Executor
        ↓
Artifacts               # named paths registered by steps
        ↓
Meeting Artifact        # ArtifactType::Meeting (canonical meeting.json + exports)
```

`MeetingRequest` must **not** carry Job knobs (`overwrite`, `max_parallel`, ASR engine, …). Those belong in `BuildOptions` (CLI / config / MCP envelope), then fold into the Job at plan time.

---

## Non-goals

- Running steps / embedding ASR / diarize / fix / translate engines
- Mixing Job fields into `MeetingRequest`
- Hard-coded “tracks only / merged only / both” runtimes
- Inventing participant names without Meeting Model or successful merge
- Second Executor beside `vd-pipeline`
- Owning Diarization Artifact schema (`vd-diarize`)
- Calling transcript branches “cleanup” (they will grow: translate, summarize, …)
- Replacing `vd-srv`

---

## Tree (target)

```
src/cli/process/vd-meeting/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs                     # plan_job() / MeetingPlanner for MCP / tests
│   ├── paths.rs
│   ├── cli/                       # flags / document → MeetingRequest + BuildOptions
│   ├── config/
│   ├── model/                     # Meeting Model + input roles
│   │   ├── mod.rs
│   │   ├── meeting.rs
│   │   └── input.rs
│   ├── planner/                   # central planning engine
│   │   ├── mod.rs                 # MeetingPlanner::plan(request, options) → Job
│   │   ├── normalize.rs           # CLI/document → ResolvedMeeting (not “analyze”)
│   │   ├── graph/                 # main graph logic
│   │   │   ├── mod.rs
│   │   │   ├── transcript.rs      # one transcript branch per recording that needs text
│   │   │   ├── diarize.rs
│   │   │   └── merge.rs           # meeting-merge capability step
│   │   ├── artifacts.rs           # wire named artifacts / ids
│   │   └── submit.rs              # hand Job to Executor (or return for dry-run)
│   └── status/                    # planner-phase progress (optional; thin)
│
└── tests/
    ├── unit/
    ├── integration/               # plan_job → golden Job DAG
    ├── e2e/
    └── fixtures/
        ├── meetings/
        ├── jobs/
        ├── audio/
        └── context/               # sample context inputs (md / …)
```

`meeting-merge` **implementation** may live in this crate as a binder library for `vd-pipeline`, or elsewhere later. The CLI path is always **plan → submit**, not a private run loop.

---

## Domain model (planned)

```rust
/// Meeting domain only — no Job / Executor knobs.
pub struct MeetingRequest {
    pub working_dir: Option<PathBuf>,
    pub inputs: Vec<InputSource>,
    pub meeting: MeetingModel,
    pub output: MeetingOutput,
}

/// How to plan and (later) run — separate from MeetingRequest.
pub struct BuildOptions {
    pub executor: ExecutorOptions,     // max_parallel, continue_on_error, resources, …
    pub transcribe: TranscribeDefaults, // engine, model, overwrite, …
}

pub struct ExecutorOptions {
    pub max_parallel: Option<u32>,
    pub continue_on_error: bool,
    pub resources: BTreeMap<String, u32>,
}

pub struct TranscribeDefaults {
    pub engine: Option<String>,
    pub model: Option<String>,
    pub overwrite: bool,
}

pub struct InputSource {
    pub role: InputRole,
    pub path: PathBuf,
    pub participant: Option<String>,
}

pub enum InputRole {
    Participant,
    Merged,
    Context,   // README, wiki, pdf, jira, slides, … — vd-assets decides how
}

pub struct MeetingModel {
    pub participants: Participants,
    pub diarization: DiarizationPolicy,  // enabled: Auto | True | False
    pub alignment: AlignmentOptions,
}

pub struct Participants {
    pub known: Vec<KnownParticipant>,
    pub expected: Option<CountBounds>,   // unnamed / unknown count — not "unknown:"
    pub constraints: Option<GroupConstraints>,
}

pub struct KnownParticipant {
    pub id: Option<String>,
    pub name: Option<String>,
    pub optional: bool,
    pub constraints: ParticipantConstraints,
}

pub struct ParticipantConstraints {
    pub gender: Option<Gender>,
    pub age: Option<AgeBounds>,       // reserved / optional later
    pub language: Option<String>,     // reserved / optional later
}

pub struct GroupConstraints {
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub genders: BTreeMap<Gender, CountBounds>,
}
```

Canonical YAML: [cli.md](cli.md#meeting-document) · product: [README.md](README.md#meeting-model).

### Meeting Artifact

First-class artifact type (shared with `vd-artifact` when wired):

```rust
ArtifactType::Meeting
```

Canonical object behind `meeting.json`; `.srt` / `.txt` / `.md` / per-participant files are **exports** of the same artifact — not separate sources of truth.

---

## Modules

| Path | Role |
|------|------|
| `cli/` | Flags + document → `MeetingRequest` **and** `BuildOptions` |
| `model/` | Input roles, Meeting Model, typed constraints |
| `planner/` | Validate · normalize · plan graph · wire artifacts · submit |
| `planner/graph/` | Transcript / diarize / merge step construction |
| `config/` | Defaults for Meeting Model + BuildOptions |
| `status/` | Optional planner-phase progress |

Depends on `vd-pipeline` (Job + Executor) and `vd-artifact` / `vd-progress`.

---

## Planner algorithm

```text
collect inputs
      ↓
normalize Meeting Model   (+ ResolvedMeeting)
      ↓
build transcript branches
      ↓
build diarization branch    (if policy + merged-like source)
      ↓
build meeting-merge capability
      ↓
resolve / wire artifacts
      ↓
submit Job                  (or return Job for dry-run / MCP)
```

Avoid “discover” — the planner does not crawl the filesystem looking for meetings; it **collects** declared inputs and **normalizes** them.

Detail: [README.md § Planner algorithm](README.md#planner-algorithm).

---

## Shared crates?

| Shared | This crate |
|--------|------------|
| `vd-pipeline` Job + Executor | Meeting Model, planner, CLI UX |
| `vd-artifact` | `ArtifactType::Meeting`, paths |
| `vd-progress` | Progress |

---

## Tests

```text
unit  →  integration  →  e2e
model →  Job DAG      →  binary (+ Executor when ready)
```

| Path | Role |
|------|------|
| `tests/unit/` | Model parse; typed constraints; normalize; id generation |
| `tests/integration/` | `plan_job(request, options)` → golden Job DAG |
| `tests/e2e/` | Binary dry-run; full gated |
| `tests/fixtures/meetings/` | MeetingRequest documents |
| `tests/fixtures/jobs/` | Expected Jobs |

### Unit

| Topic | Proof |
|-------|--------|
| input roles | `merged` / `participant` / `context`; unknown role → err |
| no modes | same planner path for any input subset |
| Meeting Model | `known` / `expected` / group + person constraints |
| typed constraints | `gender` (and reserved fields) — not free-form JSON map |
| `MeetingRequest` isolation | no `overwrite` / `max_parallel` on request |
| BuildOptions | fold into Job without polluting Meeting Model |
| diarization policy | `auto` / `true` / `false` → branch present or absent |
| alignment | nested `alignment.mode` |

### Integration

| Topic | Proof |
|-------|--------|
| participant tracks only | N **transcript** branches → `meeting-merge`; no `diarize` |
| merged only | transcript (+ diarize if auto) → `meeting-merge` |
| merged + tracks + auto | N transcript ∥ `diarize` → `meeting-merge` |
| `enabled: false` | no `diarize` even with merged |
| context | `prepare-context` when `role: context` |
| Job contract | Job validates via `vd-pipeline` resolve |
| ArtifactType::Meeting | merge step registers meeting artifact |

### E2E

| Case | Proof |
|------|--------|
| dry-run json | Job DAG matches fixture |
| missing inputs | exit 3 |
| bad model | exit 2 |
| full meeting | **optional / `#[ignore]`** unless `VD_MEETING_E2E_FULL=1` |

```bash
# once crate exists:
cargo test -p vd-meeting
./scripts/test.sh vd-meeting
```

---

## Public contract note

**MeetingRequest + BuildOptions → Job** is the product contract.  
Execution is always the shared Executor. MCP may call `MeetingPlanner::plan` and submit the same Job JSON.  
`ArtifactType::Meeting` is the canonical result type of `meeting-merge`.
