# VoxDecoder

License: [MIT](LICENSE)

## Rust / npm scripts

Toolchain + linters: see [src/cli/transcribe/vd-gigaam/RUST.md](src/cli/transcribe/vd-gigaam/RUST.md).

After clone: `npm install` (runs `prepare` → lefthook install).

| Script | What it does |
|--------|----------------|
| `npm test` | All crate/CLI tests via [`scripts/test.sh`](scripts/test.sh) |
| `./scripts/test.sh vd-gigaam` | `cargo test -p vd-gigaam` (same for `crates`, `vd-assets`, `vd-pipeline`, `vd-fix-casing`, `vd-fix-asr`, `vd-fix-terms`) |
| `npm run build:vd-gigaam` | Release binary with Metal (`--features metal`) → `target/release/vd-gigaam` |
| `npm run build:vd-gigaam:cpu` | Release binary without Metal |
| `npm run build:vd-fix-casing` | Release binary → `target/release/vd-fix-casing` |
| `npm run build:vd-fix-asr` | Release binary → `target/release/vd-fix-asr` |
| `npm run build:vd-fix-terms` | Release binary → `target/release/vd-fix-terms` |
| `npm run build:vd-assets` | Release binary → `target/release/vd-assets` |
| `npm run build:vd-pipeline` | Release binary → `target/release/vd-pipeline` |
| `npm run install:vd-gigaam` | `cargo install` into `~/.cargo/bin` (Metal enabled) |
| `npm run install:vd-fix-casing` | `cargo install` into `~/.cargo/bin` |
| `npm run install:vd-fix-asr` | `cargo install` into `~/.cargo/bin` |
| `npm run install:vd-fix-terms` | `cargo install` into `~/.cargo/bin` |
| `npm run install:vd-assets` | `cargo install` into `~/.cargo/bin` |
| `npm run install:vd-pipeline` | `cargo install` into `~/.cargo/bin` |
| `npm run lint:rust` | `cargo fmt --check` + `clippy -D warnings` |

```bash
npm test
npm run build:vd-gigaam
npm run build:vd-fix-casing
npm run build:vd-fix-asr
npm run build:vd-assets
vd-gigaam --help
vd-fix-casing --help
vd-fix-asr --help
vd-assets --help
```

Hooks ([lefthook.yml](lefthook.yml)): `commit-msg` → commitlint; `pre-commit` → `npm test`.

## Layout

| Path | Role |
|------|------|
| [`src/cli/`](src/cli/) | User-facing CLIs |
| [`src/crates/`](src/crates/) | Shared Rust libraries |
| [`src/mcp/`](src/mcp/) | MCP (TBD) |

## CLI apps

### Transcribation

See [src/cli/transcribe/](src/cli/transcribe/) — separate CLIs per model (not a universal adapter).

- [vd-gigaam](src/cli/transcribe/vd-gigaam/) — GigaAM ([structure](src/cli/transcribe/vd-gigaam/STRUCTURE.md), [cli](src/cli/transcribe/vd-gigaam/cli.md))
- vd-whisper — Whisper (TBD)

### Local cleaning

See [src/cli/fix/](src/cli/fix/) — post-process transcripts without re-running ASR.

Shared Rust libs: [`src/crates/`](src/crates/) — `vd-artifact`, `vd-output`, `vd-progress` (also used by `vd-gigaam` for progress/output). Product CLIs keep their own backends.

- [vd-fix-casing](src/cli/fix/vd-fix-casing/) — punctuation, casing, whitespace ([structure](src/cli/fix/vd-fix-casing/STRUCTURE.md), [cli](src/cli/fix/vd-fix-casing/cli.md)); optional `install ru` / `en`
- [vd-fix-asr](src/cli/fix/vd-fix-asr/) — recognition / wording fixes ([structure](src/cli/fix/vd-fix-asr/STRUCTURE.md), [cli](src/cli/fix/vd-fix-asr/cli.md)); priority `ru` + English insertions
- [vd-fix-terms](src/cli/fix/vd-fix-terms/) — canonical terminology ([structure](src/cli/fix/vd-fix-terms/STRUCTURE.md), [cli](src/cli/fix/vd-fix-terms/cli.md))

### Process

See [src/cli/process/](src/cli/process/) — prepare project docs and orchestrate multi-CLI runs.

- [vd-pipeline](src/cli/process/vd-pipeline/) — execute a Job (`transcribe` → `prepare-context?` → `fix-*`; YAML/JSON shared with future MCP) ([structure](src/cli/process/vd-pipeline/STRUCTURE.md), [cli](src/cli/process/vd-pipeline/cli.md))
- [vd-assets](src/cli/process/vd-assets/) — prepare project assets (default `.voxdecoder/`: `md/` + `terms.yml`) for `vd-fix-*` ([structure](src/cli/process/vd-assets/STRUCTURE.md), [cli](src/cli/process/vd-assets/cli.md))

### Diarization

- vd-diarize. hugging face / pyanote

### vd-meeting

### vd-unit

### vd-srv: background process

Background queue manager — [src/cli/vd-srv/](src/cli/vd-srv/)

## MCP

### vd-mcp

Rust MCP Server
