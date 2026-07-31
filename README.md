# VoxDecoder

License: [MIT](LICENSE)

## Rust

Toolchain + linters: see [cli/transcribe/vd-giga/RUST.md](cli/transcribe/vd-giga/RUST.md).

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
npm test                 # all CLI tests
npm run test:vd-giga     # vd-giga only
```

Hooks ([lefthook.yml](lefthook.yml)): `commit-msg` → commitlint; `pre-commit` → `npm test`. After clone: `npm install`.

## CLI apps

### Transcribation

See [cli/transcribe/](cli/transcribe/) — separate CLIs per model (not a universal adapter).

- [vd-giga](cli/transcribe/vd-giga/) — GigaAM ([structure](cli/transcribe/vd-giga/STRUCTURE.md), [cli](cli/transcribe/vd-giga/cli.md))
- vd-whisper — Whisper (TBD)

### Diarization

- vd-dia-hf. hugging face
- vd-dia-pn. pyanote

### vd-srv: background process

Background queue manager — [cli/vd-srv/](cli/vd-srv/)
