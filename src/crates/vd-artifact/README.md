# vd-artifact

Transcript artifact I/O for VoxDecoder CLIs.

## Owns

- Detect / load / `TextSpan` walk / write (`txt`, `json`, `jsonl`, `srt`, `vtt`, `md`)
- Types: `ArtifactType`, `Language`, `SpanId`, `TextSpan`, `FixOptions`, `FixResult`
- Platform path helpers: `paths::{config_path, cache_dir}`

## Does not own

- `.fixed.` output policy → [`vd-output`](../vd-output/)
- Progress UX → [`vd-progress`](../vd-progress/)
- Fix backends → `vd-fix-casing` / `vd-fix-asr` / …

```bash
cargo test -p vd-artifact
```
