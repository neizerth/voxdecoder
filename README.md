# VoxDecoder

Local speech-to-text, transcript cleanup, and derived artifacts.  
Three complementary executors — filter graph, capability DAG, recipe graph — sharing one artifact model.

License: [MIT](LICENSE)

## Architecture

```text
                 Media
                   │
                   ▼
           vd-preprocess
             (Filter Graph)
                   │
                   ▼
              Artifacts
                   │
                   ▼
             vd-pipeline
             (Capability DAG)
                   │
                   ▼
              Artifacts
                   │
                   ▼
          vd-postprocess
            (Recipe Graph)
                   │
                   ▼
          Derived Artifacts
```

| Level | What it executes |
|-------|------------------|
| **`vd-preprocess`** | Graph of media filters (`ffmpeg`, `deepfilternet`, …) |
| **`vd-pipeline`** | DAG of capabilities (`transcribe`, `diarize`, `meeting-merge`, `postprocess`, …) |
| **`vd-postprocess`** | Graph of recipe nodes (`LLM`, `process`, `http`, `mcp`, …) |

Job builders feed the middle layer; leaf tools own their graphs:

```text
   Job builders                 shared Job Executor
   ─────────────                ───────────────────
   vd-pipeline CLI  ─┐
   vd-meeting       ─┼─→  Job (DAG)  →  Executor
   MCP / vd-srv     ─┘         │
                               ├─ preprocess       → Filter Graph (vd-preprocess)
                               ├─ transcribe / fix-* / prepare-context
                               ├─ diarize          → vd-diarize
                               ├─ meeting-merge    → Meeting Artifact
                               └─ postprocess      → Recipe Graph (vd-postprocess)
```

Foreground: call a binary directly, or submit a Job via `vd-pipeline` / `vd-meeting`.  
Background (planned → **v1**): [`vd-srv`](src/cli/manage/vd-srv/) is the **execution engine** — queues Jobs, persists state, runs them on a Worker Pool against the **same** Job Executor.

Default project assets dir: **`.voxdecoder/`** (`md/` + `terms.yml`). Override with `$VD_PROJECT_DIR` or `VD_PROJECT_DIR=` in `.voxdecoder/env` / `.env`.

---

## Tools

### Transcribe

One binary per model family (not a multi-model adapter). Rust + Candle; no Python at runtime.

| CLI | Role | Status | Spec |
|-----|------|--------|------|
| [`vd-gigaam`](src/cli/transcribe/vd-gigaam/) | GigaAM ASR (Conformer + CTC/RNNT) | implemented | [cli](src/cli/transcribe/vd-gigaam/cli.md) · [structure](src/cli/transcribe/vd-gigaam/STRUCTURE.md) |
| `vd-whisper` | Whisper ASR | reserved | — |

Overview: [src/cli/transcribe/](src/cli/transcribe/).

```bash
vd-gigaam run -i meeting.ogg
vd-gigaam run -i meeting.ogg -m v3_e2e_ctc --device metal
```

### Process

Prepare media, project knowledge, run Jobs, diarize, build meeting Jobs, and derive artifacts from recipes.

| CLI | Role | Status | Spec |
|-----|------|--------|------|
| [`vd-pipeline`](src/cli/process/vd-pipeline/) | Universal Job Executor (+ CLI builder for single-source cleanup) | implemented | [cli](src/cli/process/vd-pipeline/cli.md) · [structure](src/cli/process/vd-pipeline/STRUCTURE.md) |
| [`vd-preprocess`](src/cli/process/vd-preprocess/) | Media filter chain → prepared media (`use: preprocess`) | implemented | [readme](src/cli/process/vd-preprocess/README.md) · [cli](src/cli/process/vd-preprocess/cli.md) · [structure](src/cli/process/vd-preprocess/STRUCTURE.md) |
| [`vd-assets`](src/cli/process/vd-assets/) | Docs/PDF/Office → `.voxdecoder/` (`md/` + `terms.yml`) | implemented | [cli](src/cli/process/vd-assets/cli.md) · [structure](src/cli/process/vd-assets/STRUCTURE.md) |
| [`vd-diarize`](src/cli/process/vd-diarize/) | Who spoke when → Diarization Artifact (`use: diarize`, local-first) | implemented | [cli](src/cli/process/vd-diarize/cli.md) · [structure](src/cli/process/vd-diarize/STRUCTURE.md) |
| [`vd-meeting`](src/cli/process/vd-meeting/) | Meeting Planner (MeetingRequest → Job → same Executor) | implemented | [cli](src/cli/process/vd-meeting/cli.md) · [structure](src/cli/process/vd-meeting/STRUCTURE.md) |
| [`vd-postprocess`](src/cli/process/vd-postprocess/) | Portable recipe graphs (`ExecutionRunner`) → derived artifacts (`use: postprocess`) | implemented | [cli](src/cli/process/vd-postprocess/cli.md) · [structure](src/cli/process/vd-postprocess/STRUCTURE.md) |

Overview: [src/cli/process/](src/cli/process/).

`vd-pipeline` capabilities (`use`) are actions, not binary names:

| `use` | Meaning | Implementation |
|-------|---------|----------------|
| `preprocess` | Media → prepared media via filter chain | `vd-preprocess` |
| `transcribe` | Audio/video → transcript | `engine: gigaam` (default); `whisper` reserved |
| `prepare-context` | Build project context | `vd-assets` |
| `fix-casing` | Presentation | `vd-fix-casing` |
| `fix-asr` | Wording / ASR repair | `vd-fix-asr` |
| `fix-terms` | Canonical terminology | `vd-fix-terms` |
| `diarize` | Speaker timeline | `vd-diarize` |
| `meeting-merge` | Meeting Artifact | stub in `vd-pipeline` (real merge later) |
| `postprocess` | Derived artifacts via user recipe graphs | `vd-postprocess` |

```bash
vd-pipeline run -i meeting.ogg
vd-pipeline run -i meeting.ogg --docs ./docs --dry-run --json
vd-pipeline run job.yaml

vd-assets run -i ./docs
vd-assets run -i ./spec.pdf --ocr

vd-diarize run -i meeting.wav
vd-diarize run -i meeting.wav --backend stub

vd-postprocess run --input meeting=meeting.json --recipe ./summary.yaml
```

| Tool | Owns |
|------|------|
| `vd-pipeline` | Shared Job Executor (capability DAG) |
| `vd-preprocess` | Filter graph + media providers → prepared media |
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

### Other / planned

| CLI | Role | Status | Spec |
|-----|------|--------|------|
| `vd-unit` | — | TBD | — |
| [`vd-srv`](src/cli/manage/vd-srv/) | Execution engine (node schedule · Resource Classes · Worker Pool · persist → shared Executor) | implemented (v1) | [readme](src/cli/manage/vd-srv/README.md) · [cli](src/cli/manage/vd-srv/cli.md) · [structure](src/cli/manage/vd-srv/STRUCTURE.md) · [transport](src/cli/manage/vd-srv/TRANSPORT.md) |
| `vd-mcp` | MCP server (same Job schema as `vd-pipeline`) | TBD | — |

---

## Shared crates

| Crate | Owns |
|-------|------|
| [`vd-artifact`](src/crates/vd-artifact/) | Artifact load/walk/write, shared types, platform `paths` |
| [`vd-output`](src/crates/vd-output/) | `-o` / `-d` / `--in-place` / `--overwrite`; caller naming |
| [`vd-progress`](src/crates/vd-progress/) | Stderr progress (`start` / `phase` / `done` / `error`) |

Overview: [src/crates/](src/crates/).

---

## Layout

| Path | Role |
|------|------|
| [`src/cli/`](src/cli/) | User-facing CLIs |
| [`src/cli/manage/`](src/cli/manage/) | Long-running / operator tools (`vd-srv`, …) |
| [`src/crates/`](src/crates/) | Shared Rust libraries |
| [`src/mcp/`](src/mcp/) | MCP (TBD) |

---

## Build / test

Toolchain + linters: see [src/cli/transcribe/vd-gigaam/RUST.md](src/cli/transcribe/vd-gigaam/RUST.md).

After clone: `npm install` (runs `prepare` → lefthook install).

| Script | What it does |
|--------|----------------|
| `npm test` | All crate/CLI tests via [`scripts/test.sh`](scripts/test.sh) |
| `./scripts/test.sh vd-pipeline` | `cargo test -p vd-pipeline` (also `vd-gigaam`, `crates`, `vd-assets`, `vd-fix-*`) |
| `npm run build:vd-*` | Release binary → `target/release/vd-*` |
| `npm run install:vd-*` | `cargo install` into `~/.cargo/bin` |
| `npm run lint:rust` | `cargo fmt --check` + `clippy -D warnings` |

```bash
npm test
npm run build:vd-gigaam
npm run build:vd-pipeline
npm run build:vd-assets
npm run build:vd-fix-casing
npm run build:vd-fix-asr
npm run build:vd-fix-terms

vd-pipeline --help
vd-gigaam --help
vd-assets --help
```

Hooks ([lefthook.yml](lefthook.yml)): `commit-msg` → commitlint; `pre-commit` → `npm test`.
