# Audio processing

## Purpose

Plan and run audio processing Jobs on the VoxDecoder Runtime via MCP (`process_audio` / `plan.audio`).

Prefer MCP tools over inventing file paths. Ask for confirmation before `execute: true`.

The default audio Job includes layout fixing (`fix-layout`) after casing / ASR / terms — readable paragraphs without changing lexical content.

Video inputs are fine: preprocess extracts the audio track with **ffmpeg** (`extract-audio`) and uses that WAV as the Job source.

**Language:** This Skill is written in English. Reply to the user in their language (or the agent's configured conversation language). Do not switch user-facing messages to English just because this document is English.

## Workflow

1. Confirm the **media** input path with the user (audio or video).
2. Ask about **accompanying documents / materials** (agendas, glossaries, name lists, PDFs, markdown, slides notes). These feed `vd-assets` via `prepare-context` and improve `fix-asr` / `fix-terms`.
   - If the user provides a folder or file of materials, pass it as `docs` on `process_audio` (absolute or Runtime-visible path).
   - If there are no materials, omit `docs` (Runtime defaults to `.`).
   - Do not dump document contents into the chat as a substitute for `docs` — point the Runtime at the files.
3. Offer a **preprocess speed** choice (shorter ASR wall time; timestamps remapped via TimeMap). Present these options and wait for the user to pick one — do not enable silently or invent a default:
   - **1.5×** → `speed: 1.5`
   - **2×** → `speed: 2`
   - **2.2×** → `speed: 2.2`
   - **No speedup** → omit `speed`
4. Call `process_audio` with `execute: true` only after confirmation (and after the speed choice). On macOS you do not need to set `device` — the Runtime defaults to Metal.
5. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Accompanying documents (`docs`)

| User provides | MCP field | What happens |
|---------------|-----------|--------------|
| Folder of PDFs / md / glossary | `docs: "/path/to/materials"` | `prepare-context` → `vd-assets` → `.voxdecoder` assets for fixers |
| Single terms / notes file | `docs: "/path/to/file"` | Same |
| Nothing | omit `docs` | Default docs root `.` |

Tell the user briefly that materials improve name/term correction; do not block the Job if they decline.

## Runtime Contract

This Skill starts long-running Runtime Jobs.

### Execution

- Start with `process_audio`.
- The tool returns a Job `id` (also called `job_id`).

### Progress

- Use `get_job` with that `id` to monitor execution until `completed`, `failed`, or `cancelled`.
- When reporting status to the user, include `progress` (0–100) and `phase` from `get_job` when present (e.g. `42% · step_start:transcribe`).
- Do not use HTTP polling (`curl`, `/health`, `/jobs/…`).
- Do not inspect Runtime sockets directly.
- Do not spam-poll `get_job`. Progress advances mainly at pipeline step boundaries; a stable percent mid-step (especially during `transcribe`) is normal.

**Ballpark duration** (wall clock, local Metal / CPU; wide variance):

- Short audio (≈1–5 min media): often **1–5 minutes** of Job time.
- Longer media / video: often **several minutes to tens of minutes**.
- Preprocess `speed` (1.5× / 2× / 2.2×) shortens ASR wall time; tell the user the estimate is rough.

**Adaptive polling** (wakeup / schedule — not a bare long `sleep`):

1. After submit, wait a **short** interval (**15–30s**), then call `get_job` once.
2. Record elapsed wall time `T` and `progress` `P` (0–100). Tell the user status (`P%` · `phase`).
3. If still `running` and `P` ≥ 1, estimate time to 100%:

   ```text
   ETA ≈ T * (100 - P) / P
   ```

   (If `P` is 0 or missing, use the ballpark above and fall back to a **60–120s** wait.)
4. Schedule the **next** `get_job` near that ETA, but clamp the wait:

   - **minimum** between polls: **30s** (never poll more often)
   - **maximum** between polls: **3 minutes** (recheck even if ETA is far / stuck)
5. Repeat from step 2 until terminal status. Recalculate ETA after every sample — do not lock the first estimate forever.
6. Only escalate after several spaced checks with no percent/`phase` change and no Runtime error.

### Results

- When the Job reaches `completed`, use `list_artifacts` with the Job `id` to discover outputs.

### Cancellation

- If the user asks to stop processing, call `cancel_job` with the Job `id`.
- Do not start a second Job when cancellation is requested.

### Failures

- If the Job fails, report the Runtime error from `get_job`.
- Do not retry automatically unless the user explicitly asks.
- Classify connectivity / MCP / Job failures per **Recovery** below — do not treat every error as “start the Runtime”.

### Recovery

If the Runtime cannot be reached (e.g. `connection refused`, `runtime unavailable`, `cannot connect to vd-srv`, `health → unavailable`):

- Ask the user whether they want to start it.
- Suggest:

```text
vdctl ensure
```

(`vdctl ensure` starts the Runtime only if it is not already running. Prefer it over unconditional `vdctl up`.)

If MCP tools are unavailable (e.g. `No MCP server configured`, `Tool not found`, `process_audio does not exist`):

- Do not suggest `vdctl up` or `vdctl ensure`.
- Ask the user to install or verify the MCP integration.
- Suggest:

```text
vdctl mcp verify
```

or

```text
vdctl mcp install
```

If a Job is already running / the Runtime answers:

- Use Runtime tools (`get_job`, `cancel_job`, `list_artifacts`).
- Never poll Runtime over HTTP.
- Do not suggest lifecycle commands.

## Examples

```text
process_audio → job_id → get_job (until completed) → list_artifacts
process_audio (speed: 1.5|2|2.2) → job_id → get_job (until completed) → list_artifacts
process_audio (docs: /path/to/materials) → …
process_audio (video.mp4) → preprocess extract-audio → …
process_audio → job_id → cancel_job
```

## Notes

Reply in the user's / agent's conversation language (see Language above).
