# VoxDecoder

Local speech-to-text and transcript cleanup. Separate CLIs per concern; one Job executor to chain them.

License: [MIT](LICENSE)

## Pipeline

```text
audio / video / docs / meeting tracks
        │
        ▼
   Job builders                 shared Executor
   ─────────────                ───────────────
   vd-pipeline CLI  ─┐
   vd-meeting       ─┼─→  Job (DAG)  →  Executor
   MCP / vd-srv     ─┘         │
                               ├─ transcribe / fix-* / prepare-context
                               ├─ diarize          → vd-diarize
                               └─ meeting-merge    → Meeting Artifact
```

Foreground: call a binary directly, or submit a Job via `vd-pipeline` / `vd-meeting`.  
Background (planned): `vd-srv` queues Jobs to the **same** Executor.

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
vd-gigaam run -i meeting.ogg -m v2_rnnt --device metal
```

### Process

Prepare project knowledge, run Jobs, diarize, and build meeting Jobs.

| CLI | Role | Status | Spec |
|-----|------|--------|------|
| [`vd-pipeline`](src/cli/process/vd-pipeline/) | Universal Job Executor (+ CLI builder for single-source cleanup) | implemented | [cli](src/cli/process/vd-pipeline/cli.md) · [structure](src/cli/process/vd-pipeline/STRUCTURE.md) |
| [`vd-assets`](src/cli/process/vd-assets/) | Docs/PDF/Office → `.voxdecoder/` (`md/` + `terms.yml`) | implemented | [cli](src/cli/process/vd-assets/cli.md) · [structure](src/cli/process/vd-assets/STRUCTURE.md) |
| [`vd-diarize`](src/cli/process/vd-diarize/) | Who spoke when → Diarization Artifact (`use: diarize`, local-first) | implemented | [cli](src/cli/process/vd-diarize/cli.md) · [structure](src/cli/process/vd-diarize/STRUCTURE.md) |
| [`vd-meeting`](src/cli/process/vd-meeting/) | Meeting Planner (MeetingRequest → Job → same Executor) | implemented | [cli](src/cli/process/vd-meeting/cli.md) · [structure](src/cli/process/vd-meeting/STRUCTURE.md) |

Overview: [src/cli/process/](src/cli/process/).

`vd-pipeline` capabilities (`use`) are actions, not binary names:

| `use` | Meaning | Implementation |
|-------|---------|----------------|
| `transcribe` | Audio/video → transcript | `engine: gigaam` (default); `whisper` reserved |
| `prepare-context` | Build project context | `vd-assets` |
| `fix-casing` | Presentation | `vd-fix-casing` |
| `fix-asr` | Wording / ASR repair | `vd-fix-asr` |
| `fix-terms` | Canonical terminology | `vd-fix-terms` |
| `diarize` | Speaker timeline | `vd-diarize` |
| `meeting-merge` | Meeting Artifact | stub in `vd-pipeline` (real merge later) |

```bash
vd-pipeline run -i meeting.ogg
vd-pipeline run -i meeting.ogg --docs ./docs --dry-run --json
vd-pipeline run job.yaml

vd-assets run -i ./docs
vd-assets run -i ./spec.pdf --ocr

vd-diarize run -i meeting.wav
vd-diarize run -i meeting.wav --backend stub
```

| Tool | Owns |
|------|------|
| `vd-pipeline` | Shared Executor (any Job DAG) |
| `vd-diarize` | One audio → anonymous speaker timeline |
| `vd-meeting` | Plan MeetingRequest → Job only — does not execute |

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

| CLI | Role | Status |
|-----|------|--------|
| `vd-unit` | — | TBD |
| `vd-srv` | Background queue / job runner | TBD |
| `vd-mcp` | MCP server (same Job schema as `vd-pipeline`) | TBD |

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
