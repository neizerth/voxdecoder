# vd-artifact

Transcript artifact I/O for VoxDecoder CLIs.

## Owns

- Detect / load / `TextSpan` walk / write (`txt`, `json`, `jsonl`, `srt`, `vtt`, `md`)
- Types: `ArtifactType`, `Language`, `SpanId`, `TextSpan`, `FixOptions`, `FixResult`
- **TimeMap** (`processed → original` timeline) + remap helpers for segments JSON / SRT
- Platform path helpers: `paths::{config_path, cache_dir}`
- Project assets dir: `paths::{project_dir, project_dir_if_present}` — default `.voxdecoder`, override `$VD_PROJECT_DIR` / `.voxdecoder/env` / `.env`

See platform ADR: [`docs/adr/0001-platform-refactoring-plan.md`](../../../docs/adr/0001-platform-refactoring-plan.md).

## Does not own

- `.fixed.` output policy → [`vd-output`](../vd-output/)
- Progress UX → [`vd-progress`](../vd-progress/)
- Fix backends → `vd-fix-casing` / `vd-fix-asr` / …

```bash
cargo test -p vd-artifact
```
