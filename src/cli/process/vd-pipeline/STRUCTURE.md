# vd-pipeline — project layout

Rust crate: Job builder (CLI) + Job Executor.

**Status: implemented.** Workspace member: `src/cli/process/vd-pipeline`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md)

---

## Philosophy

```text
CLI flags / Job file / MCP JSON  →  Job  →  Executor  →  Capabilities
```

- **Job** is the unit of work (YAML/JSON specification).
- **Executor** does not know whether the Job came from flags, a file, or MCP.
- **`use`** is an action (`transcribe`, `prepare-context`, `fix-*`), not a binary name.
- Implementation knobs live under **`options`** (`engine`, `model`, …).
- **`id`** wires artifacts; **`name`** is an optional human label only.

Domain ASR / context / fix logic stays in implementations. This crate owns schema, resolve, status, execute.

---

## Non-goals

- Second runtime path for “standard mode” (CLI only builds a Job)
- Flat flags mixed into the step root (use `options`)
- Using `name` as an artifact id
- Full DAG / `depends_on` in v1 (schema leaves room)
- Reimplement engines
- Silent `whisper` before it exists
- Replace `vd-srv`

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
│   │   ├── mod.rs              # Job, Step, ArtifactRef
│   │   ├── parse.rs
│   │   ├── default.rs          # CLI flags → default Job
│   │   └── resolve.rs          # working_dir, artifacts, engine gate
│   ├── status/                 # step / id? / name? / path
│   └── exec/
│       ├── mod.rs              # Executor::run(job)
│       └── bind.rs             # capability → implementation
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
    pub name: Option<String>,       // job label only
    pub working_dir: PathBuf,
    pub input: JobInput,
    pub context: JobContext,
    pub output: JobOutput,
    pub continue_on_error: bool,
    pub steps: Vec<Step>,
}

pub struct Step {
    pub r#use: Capability,          // Transcribe | PrepareContext | FixCasing | …
    pub id: Option<String>,         // artifact id for wiring
    pub name: Option<String>,       // optional human label — not for wiring
    pub input: Option<ArtifactRef>, // Id("transcript") | Path(...)
    pub output: Option<PathBuf>,
    pub skip: bool,
    pub options: BTreeMap<String, ArgValue>,
}

pub enum Capability {
    Transcribe,
    PrepareContext,
    FixCasing,
    FixAsr,
    FixTerms,
}

pub enum ArtifactRef {
    Id(String),
    Path(PathBuf),
}
```

Default Job from CLI: see [cli.md](cli.md#default-job-shape-what-cli-builds).

---

## Modules

| Path | Role |
|------|------|
| `cli/` | Human flags → `Job` |
| `job/` | Schema, parse, default builder, resolve |
| `status/` | Progress: `step` = capability, `id` / `name` optional, `path` = filesystem |
| `exec/` | Executor; bind capability + `options` → implementation |
| `config/` | Defaults (`progress`, `asr`, …) |

---

## Runtime

```text
build or parse Job
resolve working_dir + paths
resolve artifact ids
gate reserved engines
dry-run? → emit Job → exit 0
for step in steps:
  skipped? → status skipped; continue
  status step_start (path = resolved input)
  exec capability(options)
  remap progress → {step}:{phase}
  fail? → step_failed; maybe stop
  register artifact if id set
  status step_done (path = primary output)
done
```

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
| `tests/fixtures/audio/` | Optional short audio for transcribe e2e |

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

### Integration (Executor)

Stub each capability: record call (`use`, resolved paths, `options`) and return a fake primary output path. No real `vd-gigaam` / `vd-assets` / `vd-fix-*`.

| File | Must prove |
|------|------------|
| `exec_order.rs` | steps run in order; `skip` → `skipped` and no call |
| `exec_artifacts.rs` | `id: transcript` then `input: transcript` passes the registered path |
| `exec_chain.rs` | omitted `input` uses previous primary output |
| `exec_continue.rs` | failure stops by default; `continue_on_error` runs the rest |
| `exec_progress.rs` | emits `step_start` / `step_done` / `{step}:…` with `path` = file, `step` = capability |
| `exec_prepare_context.rs` | step present only when `context.docs` set (default Job) |
| `exec_options.rs` | `options` forwarded untouched to the binder |

### E2E (binary)

Spawn `cargo_bin!("vd-pipeline")`. Isolate with `VD_PIPELINE_CONFIG` (+ clear `VD_PROJECT_DIR`).

| File / case | Must prove |
|-------------|------------|
| `dry_run_default.rs` | `-i audio --dry-run --json` → Job with `transcribe` + fix steps; no `prepare-context` without `--docs` |
| `dry_run_with_docs.rs` | `--docs` inserts `prepare-context` |
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
VD_PIPELINE_E2E_FULL=1 cargo test -p vd-pipeline --test e2e run_full_pipeline -- --ignored
```

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

**Job + Executor** are the product. CLI is one frontend. MCP reuses the Job document. Implementation binding is an internal detail.
