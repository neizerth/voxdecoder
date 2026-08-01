# vd-progress

Stderr progress reporting for long-running CLIs (`vd-fix-*`, `vd-gigaam`, …).

## Scheme

NDJSON on stderr: `start` → `phase`* → `done` | `error`.

| Event | Role |
|-------|------|
| `start` | Optional `input` / `output` / `artifact_type` / `language` / `model` / `device` / `path` |
| `phase` | Mid-work: `phase` name + optional `percent`, `span`/`span_total`, `segment`/`segment_total`, bytes |
| `done` | Optional `output` / `model` / `path` / `duration_sec` / `char_count` |
| `error` | `code` + `message` |

Helpers: `ProgressEvent::phase`, `phase_span`, `phase_download`.

```bash
cargo test -p vd-progress
```
