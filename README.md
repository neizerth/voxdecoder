# VoxDecoder

Local speech-to-text, transcript cleanup, and derived artifacts.  
An **artifact-processing platform**: every capability is `Artifact(s) → Artifact(s)`.  
Three complementary executors — filter graph, capability DAG, recipe graph — share one model.

License: [MIT](LICENSE)

## Architecture

```text
                     Media
                       │
                       ▼
              Filter Graph
             (vd-preprocess)
                       │
             Audio + TimeMap*
                       │
                       ▼
             Capability DAG
              (vd-pipeline)
                       │
                 Artifacts
                       │
          Executor remaps timeline
              via TimeMap*
                       │
                       ▼
              Canonical Artifacts
                       │
                       ▼
               Recipe Graph
            (vd-postprocess)
                       │
                       ▼
             Derived Artifacts

* TimeMap only when timing filters rewrite the clock (speed, trim-silence, …).
```

| Level | What it executes |
|-------|------------------|
| **`vd-preprocess`** | Graph of media filters (`ffmpeg`, …) → prepared media (+ TimeMap) |
| **`vd-pipeline`** | DAG of capabilities (`transcribe`, `diarize`, `meeting-merge`, `postprocess`, …) |
| **`vd-postprocess`** | Graph of recipe nodes (`LLM`, `process`, `http`, `mcp`, …) |

Job builders and **Runtime API clients** feed the platform:

```text
Clients
  Desktop · CLI · MCP · REST · Web
        │
        ▼
Runtime API (stable)
        │
        ▼
Runtime (vd-srv)
  Planner · Scheduler · Resource Manager · Executor
        │
        ▼
Capabilities
  (lib preferred · CLI fallback)
```

| Role | Responsibility | Examples |
|------|----------------|----------|
| **Runtime API client** | Speaks only the Runtime API | **`vd-mcp`**, Desktop, Web, CLI `--via-srv`, REST/gRPC |
| **Runtime** | Domain Request → Job (Planner); schedule; resources; observe | **`vd-srv`** |
| **Executor** | Runs the capability DAG (+ TimeMap remap) | shared Executor (`vd-pipeline`) |
| **Capability** | Domain work | `vd-gigaam`, `vd-preprocess`, `vd-fix-*`, … |

Foreground without Runtime: `vd-pipeline run` / `vd-meeting` may still plan/run locally.  
Background / Docker / k8s: [`vd-srv`](src/cli/manage/vd-srv/) is the Runtime. [`vd-mcp`](src/cli/manage/vd-mcp/) only forwards Requests — Planners live in the Runtime.

Containers: [`docs/runtime.md`](docs/runtime.md).  
Build / backends: [`docs/adr/0002-build-and-container-strategy.md`](docs/adr/0002-build-and-container-strategy.md).  
Platform RFC: [`docs/adr/0001-platform-refactoring-plan.md`](docs/adr/0001-platform-refactoring-plan.md).

Default project assets dir: **`.voxdecoder/`** (`md/` + `terms.yml`). Override with `$VD_PROJECT_DIR` or `VD_PROJECT_DIR=` in `.voxdecoder/env` / `.env`.

---

## Tools

### Transcribe

One binary per model family (not a multi-model adapter). Rust + Candle; no Python at runtime.

| CLI | Role | Status | Spec |
|-----|------|--------|------|
| [`vd-gigaam`](src/cli/transcribe/vd-gigaam/) | GigaAM ASR (Conformer + CTC/RNNT) → transcript / segments / SRT | implemented | [cli](src/cli/transcribe/vd-gigaam/cli.md) · [structure](src/cli/transcribe/vd-gigaam/STRUCTURE.md) |
| `vd-whisper` | Whisper ASR | reserved | — |

Overview: [src/cli/transcribe/](src/cli/transcribe/).

```bash
vd-gigaam run -i meeting.ogg
vd-gigaam run -i meeting.ogg -m v3_e2e_ctc --device metal --segments
```

### Process

Prepare media, orchestrate Jobs, diarize, plan meetings, and derive artifacts from recipes.  
Process CLIs are graph builders/executors — not a fixed “cleanup pipeline”.

| CLI | Role | Status | Spec |
|-----|------|--------|------|
| [`vd-pipeline`](src/cli/process/vd-pipeline/) | Universal Job Executor (+ CLI Job builder for single-source work) | implemented | [cli](src/cli/process/vd-pipeline/cli.md) · [structure](src/cli/process/vd-pipeline/STRUCTURE.md) · [workflow](src/cli/process/vd-pipeline/WORKFLOW.md) |
| [`vd-preprocess`](src/cli/process/vd-preprocess/) | Media **filter graph** → prepared media (+ TimeMap when time changes); `use: preprocess` | implemented | [readme](src/cli/process/vd-preprocess/README.md) · [cli](src/cli/process/vd-preprocess/cli.md) · [structure](src/cli/process/vd-preprocess/STRUCTURE.md) |
| [`vd-assets`](src/cli/process/vd-assets/) | Docs/PDF/Office → `.voxdecoder/` (`md/` + `terms.yml`); `use: prepare-context` | implemented | [cli](src/cli/process/vd-assets/cli.md) · [structure](src/cli/process/vd-assets/STRUCTURE.md) |
| [`vd-diarize`](src/cli/process/vd-diarize/) | Who spoke when → SpeakerTimeline; `use: diarize` (local-first) | implemented | [cli](src/cli/process/vd-diarize/cli.md) · [structure](src/cli/process/vd-diarize/STRUCTURE.md) |
| [`vd-meeting`](src/cli/process/vd-meeting/) | Meeting **Planner** only: MeetingRequest → Job (does not execute) | implemented | [cli](src/cli/process/vd-meeting/cli.md) · [structure](src/cli/process/vd-meeting/STRUCTURE.md) |
| [`vd-postprocess`](src/cli/process/vd-postprocess/) | Portable **recipe graphs** (`ExecutionRunner`) → derived artifacts; `use: postprocess` | implemented | [readme](src/cli/process/vd-postprocess/README.md) · [cli](src/cli/process/vd-postprocess/cli.md) · [structure](src/cli/process/vd-postprocess/STRUCTURE.md) |

Overview: [src/cli/process/](src/cli/process/).

`vd-pipeline` capabilities (`use`) are actions, not binary names:

| `use` | Meaning | Implementation |
|-------|---------|----------------|
| `preprocess` | Media → prepared media (+ TimeMap) via filter chain | `vd-preprocess` |
| `transcribe` | Audio/video → transcript (segments / words / SRT optional) | `engine: gigaam` (default); `whisper` reserved |
| `prepare-context` | Build project context | `vd-assets` |
| `fix-casing` | Presentation | `vd-fix-casing` |
| `fix-asr` | Wording / ASR repair | `vd-fix-asr` |
| `fix-terms` | Canonical terminology | `vd-fix-terms` |
| `diarize` | Speaker timeline | `vd-diarize` |
| `meeting-merge` | Meeting Artifact | stub in `vd-pipeline` (real merge later) |
| `postprocess` | Derived artifacts via recipe graphs | `vd-postprocess` |

When a preprocess step emits a TimeMap, the Executor remaps timeline artifacts (segments, SRT, diarization, …) to the **original** media clock before registering them as canonical.

```bash
vd-pipeline run -i meeting.ogg
vd-pipeline run -i meeting.ogg --docs ./docs --dry-run --json
vd-pipeline run job.yaml

vd-preprocess run -i meeting.wav --filter 'mono' --filter 'resample:rate=16000'
vd-assets run -i ./docs
vd-diarize run -i meeting.wav --backend stub
vd-meeting run meeting.yaml --dry-run --json
vd-postprocess run --input meeting=meeting.json --recipe ./summary.yaml
```

| Tool | Owns |
|------|------|
| `vd-pipeline` | Shared Job Executor: capability DAG, artifact registry, TimeMap application, progress / report |
| `vd-preprocess` | Filter graph + media providers → prepared media (+ TimeMap sidecar) |
| `vd-assets` | Project knowledge pack under `.voxdecoder/` |
| `vd-diarize` | One audio → anonymous speaker timeline |
| `vd-meeting` | Plan MeetingRequest → Job only — does not execute |
| `vd-postprocess` | Recipe graph + `ExecutionRunner` → derived artifacts |

### Fix (local cleaning)

Post-process text without re-running ASR. Same I/O contract on all three: any text artifact in → same type out; default `{stem}.fixed.{ext}`.

```text
vd-fix-casing  →  vd-fix-asr  →  vd-fix-terms
   (form)           (words)          (terminology)
```

| CLI | Owns | Core rule | Spec |
|-----|------|-----------|------|
| [`vd-fix-casing`](src/cli/fix/vd-fix-casing/) | Presentation | Never changes words | [cli](src/cli/fix/vd-fix-casing/cli.md) · [structure](src/cli/fix/vd-fix-casing/STRUCTURE.md) |
| [`vd-fix-asr`](src/cli/fix/vd-fix-asr/) | Wording / meaning | Changes words only to restore meaning | [cli](src/cli/fix/vd-fix-asr/cli.md) · [structure](src/cli/fix/vd-fix-asr/STRUCTURE.md) |
| [`vd-fix-terms`](src/cli/fix/vd-fix-terms/) | Canonical terms | Never guesses | [cli](src/cli/fix/vd-fix-terms/cli.md) · [structure](src/cli/fix/vd-fix-terms/STRUCTURE.md) |

Overview: [src/cli/fix/](src/cli/fix/).

Shared UX: `run` / `config`, `--dry-run`, `--progress=text|json`, `--language`, priority CLI > config > default. Optional language packs via `install` / `remove` / `list` / `info` when a tool needs them.

```bash
vd-fix-casing run -i transcript.txt
vd-fix-asr run -i transcript.fixed.txt --context ./.voxdecoder
vd-fix-terms run -i transcript.fixed.txt --terms ./.voxdecoder
```

```text
мы используем гитхап экшенс
        ↓ vd-fix-asr
мы используем гитхаб экшенс
        ↓ vd-fix-terms
мы используем GitHub Actions
```

### Manage / planned

| CLI | Role | Status | Spec |
|-----|------|--------|------|
| [`vd-srv`](src/cli/manage/vd-srv/) | **Runtime** — queue · schedule · Worker Pool · persist → shared Executor | implemented (v1) | [readme](src/cli/manage/vd-srv/README.md) · [cli](src/cli/manage/vd-srv/cli.md) · [structure](src/cli/manage/vd-srv/STRUCTURE.md) · [transport](src/cli/manage/vd-srv/TRANSPORT.md) · [runtime](docs/runtime.md) |
| [`vd-mcp`](src/cli/manage/vd-mcp/) | MCP Gateway — Runtime API client (forwards Requests; no Planner) | planned | [readme](src/cli/manage/vd-mcp/README.md) · [cli](src/cli/manage/vd-mcp/cli.md) · [structure](src/cli/manage/vd-mcp/STRUCTURE.md) |
| `vd-unit` | — | TBD | — |

---

## Shared crates

| Crate | Owns |
|-------|------|
| [`vd-artifact`](src/crates/vd-artifact/) | Artifact load/walk/write; **TimeMap** + timeline remap; shared types; platform `paths` |
| [`vd-output`](src/crates/vd-output/) | `-o` / `-d` / `--in-place` / `--overwrite`; caller naming |
| [`vd-progress`](src/crates/vd-progress/) | Stderr progress (`start` / `phase` / `done` / `error`) |

Overview: [src/crates/](src/crates/).

---

## Layout

| Path | Role |
|------|------|
| [`docs/adr/`](docs/adr/) | Platform ADRs / RFCs |
| [`docs/runtime.md`](docs/runtime.md) | Runtime Environment + container images |
| [`docs/input-source.md`](docs/input-source.md) | Shared `InputSource` (`path` \| `uri` \| `artifact` \| `blob`) |
| [`docs/adr/0002-build-and-container-strategy.md`](docs/adr/0002-build-and-container-strategy.md) | Native vs Docker builds, backends, matrix |
| [`Dockerfile`](Dockerfile) | One build → `runtime` / `mcp` targets |
| [`src/cli/`](src/cli/) | User-facing CLIs |
| [`src/cli/process/`](src/cli/process/) | Filter / Job / recipe / meeting tools |
| [`src/cli/manage/`](src/cli/manage/) | Runtime (`vd-srv`) · MCP Gateway (`vd-mcp`, planned) · other operator tools |
| [`src/crates/`](src/crates/) | Shared Rust libraries |

---

## Build / test

Toolchain + linters: see [src/cli/transcribe/vd-gigaam/RUST.md](src/cli/transcribe/vd-gigaam/RUST.md).

After clone: `npm install` (runs `prepare` → lefthook install).

| Script | What it does |
|--------|----------------|
| `npm test` | All crate/CLI tests via [`scripts/test.sh`](scripts/test.sh) |
| `npm run build` | Release Runtime set ([`scripts/build.sh`](scripts/build.sh); Metal on macOS — [ADR 0002](docs/adr/0002-build-and-container-strategy.md)) |
| `npm run build:cpu` | Same set, CPU features only (matches Docker) |
| `./scripts/test.sh vd-pipeline` | `cargo test -p vd-pipeline` (also `vd-gigaam`, `crates`, `vd-assets`, `vd-fix-*`, …) |
| `npm run build:vd-*` | Single release binary → `target/release/vd-*` |
| `npm run install:vd-*` | `cargo install` into `~/.cargo/bin` |
| `npm run lint:rust` | `cargo fmt --check` + `clippy -D warnings` |

```bash
npm test
npm run build                 # Runtime set (Metal on macOS; CPU elsewhere)
npm run build:cpu             # same packages, CPU only (Docker parity)
npm run build:vd-gigaam       # gigaam alone + Metal feature
npm run build:vd-pipeline
npm run build:vd-preprocess
npm run build:vd-postprocess
npm run build:vd-assets
npm run build:vd-fix-casing
npm run build:vd-fix-asr
npm run build:vd-fix-terms

vd-pipeline --help
vd-preprocess --help
vd-gigaam --help
vd-assets --help
```

Hooks ([lefthook.yml](lefthook.yml)): `commit-msg` → commitlint; `pre-commit` → `npm test`.
