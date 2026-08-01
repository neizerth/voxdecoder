# Process CLIs

Local document / Job orchestration tools.

```text
CLI / Job file

        ↓

    vd-pipeline   (Job → Executor)

        ↓

transcribe → prepare-context? → fix-casing → fix-asr → fix-terms
```

| CLI | Role | Spec |
|-----|------|------|
| `vd-pipeline` | Execute a Job (CLI builds default Job, or load YAML/JSON); MCP-ready schema | [vd-pipeline/](vd-pipeline/) ([cli](vd-pipeline/cli.md), [structure](vd-pipeline/STRUCTURE.md)) |
| `vd-assets` | Implementation for `prepare-context` (`.voxdecoder/`) | [vd-assets/](vd-assets/) ([cli](vd-assets/cli.md), [structure](vd-assets/STRUCTURE.md)) |

Default project dir: **`.voxdecoder/`**. Shared via [`vd-artifact::paths`](../../crates/vd-artifact/).

Transcribe: [../transcribe/](../transcribe/). Fix: [../fix/README.md](../fix/README.md). Queue: [`vd-srv`](../vd-srv/).

```bash
vd-pipeline run -i meeting.ogg --docs ./docs
vd-pipeline run job.yaml
```
