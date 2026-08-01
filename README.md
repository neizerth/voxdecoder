# VoxDecoder

License: [MIT](LICENSE)

## Rust / npm scripts

Toolchain + linters: see [cli/transcribe/vd-gigaam/RUST.md](cli/transcribe/vd-gigaam/RUST.md).

After clone: `npm install` (runs `prepare` → lefthook install).

| Script | What it does |
|--------|----------------|
| `npm test` | All CLI tests (`test:vd-gigaam` + `test:vd-fix-casing`) |
| `npm run test:vd-gigaam` | `cargo test -p vd-gigaam` |
| `npm run test:vd-fix-casing` | `cargo test -p vd-fix-casing` |
| `npm run build:vd-gigaam` | Release binary with Metal (`--features metal`) → `target/release/vd-gigaam` |
| `npm run build:vd-gigaam:cpu` | Release binary without Metal |
| `npm run build:vd-fix-casing` | Release binary → `target/release/vd-fix-casing` |
| `npm run install:vd-gigaam` | `cargo install` into `~/.cargo/bin` (Metal enabled) |
| `npm run install:vd-fix-casing` | `cargo install` into `~/.cargo/bin` |
| `npm run lint:rust` | `cargo fmt --check` + `clippy -D warnings` |

```bash
npm test
npm run build:vd-gigaam
npm run build:vd-fix-casing
vd-gigaam --help
vd-fix-casing --help
```

Hooks ([lefthook.yml](lefthook.yml)): `commit-msg` → commitlint; `pre-commit` → `npm test`.

## CLI apps

### Transcribation

See [cli/transcribe/](cli/transcribe/) — separate CLIs per model (not a universal adapter).

- [vd-gigaam](cli/transcribe/vd-gigaam/) — GigaAM ([structure](cli/transcribe/vd-gigaam/STRUCTURE.md), [cli](cli/transcribe/vd-gigaam/cli.md))
- vd-whisper — Whisper (TBD)

### Local cleaning

See [cli/fix/](cli/fix/) — post-process transcripts without re-running ASR.

- [vd-fix-casing](cli/fix/vd-fix-casing/) — punctuation, casing, whitespace ([structure](cli/fix/vd-fix-casing/STRUCTURE.md), [cli](cli/fix/vd-fix-casing/cli.md)); optional `install ru` / `en`
- `vd-fix-asr` — recognition / wording fixes
- `vd-fix-terms` — dictionary-based term normalization

### Diarization

- vd-dia. hugging face / pyanote

### vd-meeting

### vd-srv: background process

Background queue manager — [cli/vd-srv/](cli/vd-srv/)

## MCP

### vd-mcp

Rust MCP Server