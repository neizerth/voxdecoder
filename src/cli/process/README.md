# Process CLIs

Local media preparation, Job orchestration, meeting reconstruction, and derived artifacts.

Three complementary executors:

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

```text
planners / builders     shared Job Executor
─────────────────       ───────────────────
vd-pipeline CLI  ─┐
vd-meeting       ─┼─→  Job (DAG)  →  Executor  →  capabilities
MCP / vd-srv     ─┘

vd-preprocess    ← preprocess (CLI ≡ capability; filter graph required)
vd-assets        ← prepare-context
vd-diarize       ← diarize (CLI ≡ capability)
vd-postprocess   ← postprocess (CLI ≡ capability; recipe graphs required)
```

| CLI | Role | Spec |
|-----|------|------|
| `vd-pipeline` | Universal Job Executor (+ CLI Job builder for single-source cleanup) | [vd-pipeline/](vd-pipeline/) ([cli](vd-pipeline/cli.md), [structure](vd-pipeline/STRUCTURE.md)) |
| `vd-preprocess` | Media **filter graph** → prepared media (`use: preprocess`) | [vd-preprocess/](vd-preprocess/) ([readme](vd-preprocess/README.md), [cli](vd-preprocess/cli.md), [structure](vd-preprocess/STRUCTURE.md)) |
| `vd-assets` | Implementation for `prepare-context` (`.voxdecoder/`) | [vd-assets/](vd-assets/) ([cli](vd-assets/cli.md), [structure](vd-assets/STRUCTURE.md)) |
| `vd-diarize` | Who spoke when → Diarization Artifact (`use: diarize`, local-first) | [vd-diarize/](vd-diarize/) ([cli](vd-diarize/cli.md), [structure](vd-diarize/STRUCTURE.md)) |
| `vd-meeting` | Meeting **Planner** (MeetingRequest → Job → same Executor) | [vd-meeting/](vd-meeting/) ([cli](vd-meeting/cli.md), [structure](vd-meeting/STRUCTURE.md)) |
| `vd-postprocess` | Portable **recipe graphs** (`ExecutionRunner`; `CLI > Job > Config > Recipe`) → derived artifacts | [vd-postprocess/](vd-postprocess/) ([cli](vd-postprocess/cli.md), [structure](vd-postprocess/STRUCTURE.md)) |

Default project dir: **`.voxdecoder/`**. Shared via [`vd-artifact::paths`](../../crates/vd-artifact/).

Transcribe: [../transcribe/](../transcribe/). Fix: [../fix/README.md](../fix/README.md). Queue: [`vd-srv`](../manage/vd-srv/).

```bash
vd-pipeline run -i meeting.ogg --docs ./docs
vd-pipeline run job.yaml
vd-preprocess run -i meeting.wav --chain prepare.yaml
vd-diarize run -i meeting.wav
vd-meeting run meeting.yaml --dry-run --json
vd-postprocess run --input meeting=meeting.json --recipe ./summary.yaml
```

Leaf abstractions:

| Tool | Unit of work | Who runs a step |
|------|--------------|-----------------|
| `vd-preprocess` | **Filter** in a graph | media **provider** (`ffmpeg`, `deepfilternet`, …) |
| `vd-postprocess` | **Node** in a recipe graph | **`ExecutionRunner`** (`openai`, `process`, `mcp`, …) |
