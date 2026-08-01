# vd-pipeline — execute a Job

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI / Job surface: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [../../fix/README.md](../../fix/README.md) · [../../transcribe/](../../transcribe/).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented.** Workspace member: `src/cli/process/vd-pipeline`.

## Core rule

```text
vd-pipeline executes a Job.

CLI and MCP are frontends.
The Executor only sees a Job — never “CLI mode” vs “file mode”.
```

```text
CLI flags
Job file
MCP JSON

        ↓

       Job

        ↓

    Executor

        ↓

  Capabilities
```

| Layer | Role |
|-------|------|
| **CLI** (`vd-pipeline`) | Human UX: flags → Job |
| **Job** (YAML/JSON) | Single task specification (also MCP payload) |
| **Executor** | Runs the Job; binds capabilities to implementations |

## Capabilities (`use`)

`use` names an **action**, not a binary:

| `use` | Meaning | Implementation |
|-------|---------|----------------|
| `transcribe` | Audio/video → transcript | `engine: gigaam` (default), `whisper` (**reserved**) |
| `prepare-context` | Build project context (`.voxdecoder`) | `vd-assets` |
| `fix-casing` | Presentation | `vd-fix-casing` |
| `fix-asr` | Wording / ASR repair | `vd-fix-asr` |
| `fix-terms` | Canonical terminology | `vd-fix-terms` |

Engine-specific knobs live under `options:` so reserved step fields stay free (`id`, `name`, `input`, `when`, `retry`, …).

## Artifacts vs labels

| Field | Role |
|-------|------|
| `id` | Artifact id for wiring (`input: transcript`) |
| `name` | Optional human label for UI / logs / progress. **Omit when unset** |

## Default Job (from CLI)

```bash
vd-pipeline run -i meeting.ogg
vd-pipeline run -i meeting.ogg --asr gigaam -m v2_rnnt --docs ./docs
```

builds the **same** Job a file would describe, then hands it to the Executor.

## Job file

```bash
vd-pipeline run job.yaml
vd-pipeline run job.json
```

## Progress

Always: which step, index/total, status, and how far the current capability has gotten. See [cli.md](cli.md#progress--status).

## Guarantees

`vd-pipeline` never:

- owns ASR / context / fix domain logic (implementations do)
- pretends `whisper` works before it exists
- runs a second path beside the Job Executor
- replaces `vd-srv` (queue) — this is a foreground Job runner

Full schema, CLI flags, exit codes: [cli.md](cli.md).
