# vd-pipeline — project layout

Rust crate: Job builders (CLI) + **universal Job Executor** (DAG).

**Status: implemented.** Workspace member: `src/cli/process/vd-pipeline`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md) · [../vd-meeting/](../vd-meeting/)

---

## Philosophy

```text
Any Job builder  →  Job (DAG)  →  Executor  →  Capabilities
```

- **Job** is the unit of work (YAML/JSON DAG).
- **Executor** does not know whether the Job came from flags, a file, `vd-meeting`, MCP, or `vd-srv`.
- **`use`** is an action (`preprocess`, `transcribe`, `prepare-context`, `fix-*`, `diarize`, `meeting-merge`, `postprocess`), not a binary name.
- Implementation knobs live under **`options`**.
- **`id`** / **`outputs`** register named artifacts; **`name`** is display only.
- **`inputs`** (sugar: **`input`**) and **`depends`** form DAG edges.
- **`max_parallel`** + **resource groups** limit concurrency.

Domain logic stays in implementations. This crate owns schema, resolve, schedule, status, execute.

---

## Non-goals

- Second runtime path for “standard mode” (CLI only builds a Job)
- Flat flags mixed into the step root (use `options`)
- Using `name` as an artifact id
- Reimplement engines / meeting merge / diarization inside this crate
- Silent reserved capabilities (`whisper` before it exists; `meeting-merge` is stubbed)
- Replace `vd-srv` (queue submits Jobs to this Executor)
- Let `vd-meeting` run its own executor

---

## Tree (target)

```
src/cli/process/vd-pipeline/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs                  # Executor API for MCP / tests
│   ├── paths.rs
│   ├── cli/                    # flags → Job
│   ├── config/
│   ├── job/
│   │   ├── mod.rs              # Job, WorkflowNode, Step, ArtifactRef
│   │   ├── parse.rs
│   │   ├── default.rs          # CLI flags → default Job
│   │   └── resolve.rs          # compile tree → WorkflowPlan + leaves
│   ├── artifacts.rs            # ArtifactRegistry (produces/consumes)
│   ├── meeting_artifact.rs     # Meeting / SpeakerTimeline schema stubs
│   ├── report/                 # ExecutionReport (+ critical_path / efficiency)
│   ├── status/                 # live progress (vd-progress) — no timings
│   └── exec/
│       ├── mod.rs              # recursive sequence/parallel Executor
│       └── bind.rs
│
└── tests/
    ├── unit/                   # pure Job / resolve / status (no child binaries)
    ├── integration/            # Executor + fake/stub capabilities
    ├── e2e/                    # real `vd-pipeline` binary (+ real children where feasible)
    └── fixtures/
        ├── jobs/               # valid / invalid Job yaml+json
        ├── audio/              # tiny clip for e2e transcribe (optional / ignored if missing)
        └── docs/               # tiny markdown for prepare-context
```

---

## Domain model

```rust
pub struct Job {
    pub version: u32,
    pub name: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub input: JobInput,
    pub context: JobContext,
    pub output: JobOutput,
    pub continue_on_error: bool,
    pub max_parallel: Option<u32>,
    pub resources: BTreeMap<String, u32>,  // gpu / cpu / io → slots
    pub steps: Vec<Step>,
}

pub struct Step {
    pub r#use: Capability,
    pub id: Option<String>,              // primary artifact name
    pub name: Option<String>,            // display only
    pub input: Option<String>,           // sugar → inputs
    pub inputs: Vec<String>,             // artifact ids | paths
    pub output: Option<PathBuf>,         // primary path sugar
    pub outputs: BTreeMap<String, PathBuf>,
    pub depends: Vec<String>,            // step ids (ordering)
    pub skip: bool,
    pub resource: Option<String>,
    pub options: BTreeMap<String, ArgValue>,
}

pub enum Capability {
    Preprocess,     // → vd-preprocess (filter chain required)
    Transcribe,
    PrepareContext,
    FixCasing,
    FixAsr,
    FixTerms,
    Diarize,        // → vd-diarize
    MeetingMerge,   // stub binder → meeting.json
    Postprocess,    // → vd-postprocess (recipes required)
}
}
```

Default Job from CLI: see [cli.md](cli.md#default-job-shape-cli).

---

## Modules

| Path | Role |
|------|------|
| `cli/` | Human flags → `Job` |
| `job/` | Schema, parse, default builder, resolve |
| `status/` | Progress: `step` = capability, `id` / `name` optional, `path` = filesystem (no timings) |
| `report/` | `ExecutionReport`: per-step `duration_ms`, status, backend/model, I/O stats |
| `exec/` | Schedule + invoke binders; always builds a report |
| `exec/` | Executor; bind capability + `options` → implementation |
| `config/` | Defaults (`progress`, `asr`, …) |

---

## Runtime

```text
build or parse Job
normalize input → inputs; resolve working_dir
validate artifact refs + DAG (no cycles)
gate reserved engines / capabilities
dry-run? → emit Job → exit 0
schedule:
  while incomplete:
    ready = steps whose inputs/depends are satisfied
    run up to max_parallel within resource caps
    for each finished step:
      register id + outputs into artifact map
      emit progress
done
```

Linear Jobs remain valid DAGs (single chain). Parallelism is an Executor concern — builders only declare edges.

---

## Shared crates?

| Shared | This crate |
|--------|------------|
| `vd-progress`, `vd_artifact::paths` | Job schema, Executor, CLI UX |

---

## Tests

Testing is part of the product contract for `vd-pipeline`: the Job must be the **same** whether built from CLI or a file, and the Executor must behave identically for that Job. Prefer failing tests over “it worked once manually”.

```text
unit  →  integration  →  e2e
Job   →  Executor     →  binary + real children
```

### Layout

| Path | Role |
|------|------|
| `tests/unit/` | No process spawn; Job parse / default / resolve / status math |
| `tests/integration/` | `Executor::run` with **stub** capability binders (deterministic, fast) |
| `tests/e2e/` | Spawn `vd-pipeline` binary; dry-run always; full run when children + fixtures exist |
| `tests/fixtures/jobs/` | Golden Jobs (yaml + json), invalid Jobs |
| `tests/fixtures/docs/` | Minimal docs for `prepare-context` |
| `tests/fixtures/audio/` | Hobby dialogue + sauna text clips + `.expected.txt` for gated full ASR e2e |

Cargo test targets:

```toml
[[test]]
name = "unit"
path = "tests/unit/mod.rs"

[[test]]
name = "integration"
path = "tests/integration/mod.rs"

[[test]]
name = "e2e"
path = "tests/e2e/mod.rs"
```

### Unit

| File | Must prove |
|------|------------|
| `cli.rs` | shorthand `-i`; `--asr` / `-m` / `--docs`; job-file vs `-i` conflict → exit 2 |
| `job_parse.rs` | yaml ↔ json round-trip; `options` vs reserved (`id`, `name`, `input`); unknown `use` → err |
| `default_job.rs` | CLI flags → **byte-equal** Job to `fixtures/jobs/default_*.{yaml,json}` |
| `artifacts.rs` | `id` / `input: id` resolve; `name` never used for wiring; unknown id → err |
| `resolve.rs` | `working_dir` + relative paths; omit `input` → previous output |
| `engine_gate.rs` | `engine: whisper` → clear reserved error before exec |
| `status.rs` | overall percent; omit unset `id` / `name` in events |
| `report.rs` | RFC3339 helper; backend/model extraction; report JSON shape |

### Integration (Executor)

Stub each capability: record call (`use`, resolved paths, `options`) and return a fake primary output path. No real `vd-gigaam` / `vd-assets` / `vd-fix-*`.

| File | Must prove |
|------|------------|
| `exec_order.rs` | steps run in order; `skip` → `skipped` and no call |
| `exec_artifacts.rs` | `id: transcript` then `input: transcript` passes the registered path |
| `exec_chain.rs` | omitted `input` uses previous primary output |
| `exec_continue.rs` | failure stops by default; `continue_on_error` runs the rest |
| `exec_progress.rs` | emits `step_start` / `step_done` / `{step}:…` with `path` = file, `step` = capability |
| `exec_prepare_context.rs` | default Job always invokes `prepare-context` |
| `exec_options.rs` | `options` forwarded untouched to the binder |
| `exec_report.rs` | `ExecutionReport` has step order, backend/model, timings; failure returns partial report |

### E2E (binary)

Spawn `cargo_bin!("vd-pipeline")`. Isolate with `VD_PIPELINE_CONFIG` (+ clear `VD_PROJECT_DIR`).

| File / case | Must prove |
|-------------|------------|
| `dry_run_default.rs` | `-i audio --dry-run --json` → Job with `transcribe` + `prepare-context` + fix steps; `context.docs` defaults to `.` |
| `dry_run_with_docs.rs` | `--docs` sets `context.docs` |
| `dry_run_file.rs` | `run fixtures/jobs/full.yaml --dry-run --json` matches resolved shape |
| `cli_equals_file.rs` | CLI-built dry-run Job ≡ same fixture Job (canonical json) |
| `whisper_exit_2.rs` | `--asr whisper` or `engine: whisper` → exit 2, stderr mentions reserved |
| `bad_job_exit_2.rs` | unknown `use` / unknown option key / unknown `input` id |
| `missing_input_exit_3.rs` | no `-i` and no file → exit 3 |
| `progress_json.rs` | `--progress=json`: NDJSON has `step` + filesystem `path` |
| `run_fix_only.rs` | Job without transcribe: txt → fix-* with shipping lexicon (needs built fix CLIs) |
| `run_prepare_context.rs` | `prepare-context` on `fixtures/docs` writes `.voxdecoder/` (`terms.yml` + `md/`) |
| `run_full_pipeline.rs` | **optional / `#[ignore]`** unless `VD_PIPELINE_E2E_FULL=1` and audio + gigaam available |

Full ASR e2e must not block default `cargo test` / CI. Gate:

```bash
VD_PIPELINE_E2E_FULL=1 cargo test --release -p vd-pipeline --test e2e run_full_pipeline -- --ignored
```

Uses `fixtures/audio/*.mp3` + converted `v3_e2e_ctc` under `vd-gigaam/models` (`VD_GIGAAM_MODELS_DIR`). Prefer `--release` (debug ASR on this clip can take >10m). When the sibling `vd-gigaam` was built with `--features metal`, the job sets `device: metal`; otherwise the same job runs without that option (CPU/`auto`).

### What “works” means

| Layer | Pass criteria |
|-------|----------------|
| Unit | Job schema + wiring invariants; no I/O side effects |
| Integration | Executor semantics vs stubs (order, artifacts, errors, progress fields) |
| E2E dry-run | Real binary emits the contract Job / exit codes |
| E2E light run | Real `prepare-context` + fix-* on fixtures |
| E2E full | Optional: real transcribe + cleanup on a short clip |

### Commands

```bash
cargo test -p vd-pipeline
cargo test -p vd-pipeline --test unit
cargo test -p vd-pipeline --test integration
cargo test -p vd-pipeline --test e2e
./scripts/test.sh vd-pipeline
```

Wire `vd-pipeline` into [`scripts/test.sh`](../../../../scripts/test.sh) (`all` + dedicated target).

---

## Public contract note

**Job + Executor** are the product. CLI and `vd-meeting` are Job builders. MCP / `vd-srv` reuse the Job document. Implementation binding is an internal detail.
