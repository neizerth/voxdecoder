# VoxDecoder

License: [MIT](LICENSE)

## Rust / npm scripts

Toolchain + linters: see [cli/transcribe/vd-giga/RUST.md](cli/transcribe/vd-giga/RUST.md).

After clone: `npm install` (runs `prepare` → lefthook install).

| Script | What it does |
|--------|----------------|
| `npm test` | All CLI tests (currently `test:vd-giga`) |
| `npm run test:vd-giga` | `cargo test -p vd-giga` |
| `npm run build:vd-giga` | Release binary with Metal (`--features metal`) → `target/release/vd-giga` |
| `npm run build:vd-giga:cpu` | Release binary without Metal |
| `npm run install:vd-giga` | `cargo install` into `~/.cargo/bin` (Metal enabled) |
| `npm run lint:rust` | `cargo fmt --check` + `clippy -D warnings` |

```bash
npm test
npm run build:vd-giga
vd-giga --help   # after install:vd-giga, or use ./target/release/vd-giga
```

Hooks ([lefthook.yml](lefthook.yml)): `commit-msg` → commitlint; `pre-commit` → `npm test`.

## CLI apps

### Transcribation

See [cli/transcribe/](cli/transcribe/) — separate CLIs per model (not a universal adapter).

- [vd-giga](cli/transcribe/vd-giga/) — GigaAM ([structure](cli/transcribe/vd-giga/STRUCTURE.md), [cli](cli/transcribe/vd-giga/cli.md))
- vd-whisper — Whisper (TBD)

### Diarization

- vd-dia. hugging face / pyanote

### vd-srv: background process

Background queue manager — [cli/vd-srv/](cli/vd-srv/)

## MCP

### vd-mcp

Rust MCP Server