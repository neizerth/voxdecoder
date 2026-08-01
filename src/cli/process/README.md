# Process CLIs

Local document / Job orchestration and meeting reconstruction.

```text
planners / builders     shared Executor
─────────────────       ───────────────
vd-pipeline CLI  ─┐
vd-meeting       ─┼─→  Job (DAG)  →  Executor  →  capabilities
MCP / vd-srv     ─┘

vd-assets     ← prepare-context
vd-diarize    ← diarize (CLI ≡ capability)
```

| CLI | Role | Spec |
|-----|------|------|
| `vd-pipeline` | Universal Job Executor (+ CLI Job builder for single-source cleanup) | [vd-pipeline/](vd-pipeline/) ([cli](vd-pipeline/cli.md), [structure](vd-pipeline/STRUCTURE.md)) |
| `vd-assets` | Implementation for `prepare-context` (`.voxdecoder/`) | [vd-assets/](vd-assets/) ([cli](vd-assets/cli.md), [structure](vd-assets/STRUCTURE.md)) |
| `vd-diarize` | Who spoke when → Diarization Artifact (`use: diarize`, local-first) | [vd-diarize/](vd-diarize/) ([cli](vd-diarize/cli.md), [structure](vd-diarize/STRUCTURE.md)) |
| `vd-meeting` | Meeting **Planner** (MeetingRequest → Job → same Executor) | [vd-meeting/](vd-meeting/) ([cli](vd-meeting/cli.md), [structure](vd-meeting/STRUCTURE.md)) — **planned** |

Default project dir: **`.voxdecoder/`**. Shared via [`vd-artifact::paths`](../../crates/vd-artifact/).

Transcribe: [../transcribe/](../transcribe/). Fix: [../fix/README.md](../fix/README.md). Queue: [`vd-srv`](../vd-srv/).

```bash
vd-pipeline run -i meeting.ogg --docs ./docs
vd-pipeline run job.yaml
vd-diarize run -i meeting.wav
# planned:
# vd-meeting run meeting.yaml
```
