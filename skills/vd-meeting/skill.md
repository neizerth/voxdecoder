# Meeting assistant

## Purpose

Meeting workflows on the VoxDecoder Runtime via MCP (`process_meeting` / `plan.meeting`): ingest audio/video/transcripts and produce structured meeting artifacts.

Keep Skills independent from Runtime internals — call MCP tools only.

Video tracks are supported: preprocess extracts audio with **ffmpeg** before ASR / diarization.

**Language:** This Skill is written in English. Reply to the user in their language (or the agent's configured conversation language). Do not switch user-facing messages to English just because this document is English.

## Workflow

1. Collect file paths the user wants processed (folder listing or explicit paths).
2. **Classify inputs by filename** (see **Filename heuristics** below) into:
   - shared mix → `role: room` (alias `merged`)
   - per-speaker tracks → `role: participant` + `participant: <id>`
   - accompanying docs/materials → `role: context`
3. Infer **speaker gender** from names in filenames when the user’s prompt does not state it (see **Gender**). Confirm uncertain guesses before execute.
4. If a **shared mix** exists and the user did **not** explicitly enable or disable diarization, **propose diarization** (`meeting.diarization.enabled: true` or `auto`) and wait for confirmation.
5. Confirm the assembled `inputs` + meeting model with the user.
6. Call `process_meeting` with `execute: true` only after confirmation.
7. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Filename heuristics

Apply case-insensitively to the **basename** (ignore extension). Prefer explicit user labels over heuristics.

### Shared mix (`role: room` / `merged`)

Treat as the common room recording when the name contains tokens such as:

`mix`, `mixed`, `merged`, `all`, `room`, `full`, `combined`, `common`, `overall`, `together`, `весь`, `общ`, `микс`, `слит`, `полный`

Examples: `meeting_mix.wav`, `all.mp4`, `merged_track.m4a`, `общая_запись.wav`.

### Per-speaker (`role: participant`)

If the basename looks like a **person name** (or contains one) and is **not** a mix token:

- Set `role: participant`.
- Set `participant` to a stable id (prefer slug of the name: `alice`, `ivan_petrov`).
- Put a display `name` under `meeting.participants.known`.

Examples: `Alice.wav`, `ivan-petrov.mp3`, `Мария_Смирнова.m4a`.

If both a mix and speaker files exist, assign roles accordingly — do not treat the mix as a participant.

### Context materials (`role: context`)

Documents and non-media materials for **vd-assets** (glossaries, agendas, attendee lists, PDFs, markdown):

- Pass as an `inputs[]` entry with `role: context` and `path` to the folder or file.
- Ask the user for materials if they mentioned slides/docs but did not attach paths.
- Do not put PDF/DOCX contents into chat instead of `role: context`.

## Gender

When a participant file (or known name) is present and the user prompt does **not** explicitly give gender:

1. Infer `male` / `female` / leave unset from the **given name** using common language conventions (RU/EN and other languages you know).
2. Set `meeting.participants.known[].constraints.gender` only when reasonably confident.
3. If unsure, ask once; do not invent gender for ambiguous nicknames (`Alex`, `Саша`, `Женя`) without confirmation.
4. Never override an explicit gender from the user prompt.

## Diarization

| Situation | Action |
|-----------|--------|
| Shared mix present; user did not mention diarization | **Propose** enabling diarization; explain it labels speakers on the mix. Wait for yes/no. |
| User asked for diarization / speaker labels on the mix | Set `meeting.diarization.enabled: true` (or `auto`). |
| User said no diarization / “only tracks” | Set `enabled: false`. |
| Only per-speaker tracks, no mix | Usually skip diarization; still confirm if ambiguous. |

Default Runtime policy is `auto` when unset — still ask when a mix is present so the user knows the Job may run diarize.

## Accompanying documents

| Role | MCP | Effect |
|------|-----|--------|
| `context` | `inputs[].role: context` | `prepare-context` → vd-assets → terms/names for fixers |

## Runtime Contract

This Skill starts long-running Runtime Jobs.

### Execution

- Start with `process_meeting`.
- The tool returns a Job `id` (also called `job_id`).

### Progress

- Use `get_job` with that `id` to monitor execution until `completed`, `failed`, or `cancelled`.
- When reporting status to the user, include `progress` (0–100) and `phase` from `get_job` when present (e.g. `42% · step_start:transcribe`).
- Do not use HTTP polling (`curl`, `/health`, `/jobs/…`).
- Do not inspect Runtime sockets directly.
- Do not spam-poll `get_job`. Progress advances mainly at pipeline step boundaries; a stable percent mid-step (especially during `transcribe` / diarize) is normal.

**Ballpark duration** (wall clock, local Metal / CPU; wide variance):

- Short inputs: often **a few minutes** of Job time.
- Full meetings (long audio + merge / diarize): often **several minutes to tens of minutes**.
- Tell the user the estimate is rough.

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

If MCP tools are unavailable (e.g. `No MCP server configured`, `Tool not found`, `process_meeting does not exist`):

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
# Mix + speakers + docs
process_meeting inputs=[room:mix.wav, participant:alice.wav, context:./docs] → …
# Video mix → extract-audio inside preprocess
process_meeting inputs=[room:meeting.mp4] + diarization → …
process_meeting → job_id → get_job (until completed) → list_artifacts
process_meeting → job_id → cancel_job
```

## Notes

Reply in the user's / agent's conversation language (see Language above).
