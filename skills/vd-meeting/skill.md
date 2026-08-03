# Meeting assistant

## Purpose

Meeting workflows on the VoxDecoder Runtime via MCP (`process_meeting` / `plan.meeting`): ingest audio/video/transcripts and produce structured meeting artifacts.

Keep Skills independent from Runtime internals — call MCP tools only.

Video tracks are supported: preprocess extracts audio with **ffmpeg** before ASR / diarization.

**Online URLs** (YouTube, direct media links, …) are first-class for media roles (`room` / `merged` / `participant`). Pass them as `inputs[].url`. The Runtime resolves each URL into local artifacts **before** DAG build (ADR 0008 / `vd-input`). You do **not** invent download steps or call `vd-url` yourself. **`role: context` cannot use `url`** — docs stay on `path` / `uri`.

**Language:** This Skill is written in English. Reply to the user in their language (or the agent's configured conversation language). Do not switch user-facing messages to English just because this document is English.

## Input recognition

Each media input uses exactly one of:

| User gives | MCP field on `inputs[]` |
|------------|-------------------------|
| Local audio / video file | `path: "/abs/or/runtime/path"` |
| `file://…` | `uri: "file://…"` |
| YouTube / http(s) media URL | `url: "https://…"` |
| Prior Runtime artifact id | `artifact: "…"` |

Convenience: a single shared recording may also be `audio.url` / `audio.path` (Runtime treats it as `role: room`).

### When the user pastes only a link

If the user gives a YouTube / http(s) media URL and **no** local file:

1. Treat it as `inputs[].url` (usually `role: room`) — do **not** ask them to download first.
2. Confirm the URL and role with the user.
3. Optionally ask about subtitles for YouTube-like sources: `ignore` (default) · `prefer` · `require` → per-input `subtitles`.
4. Continue with classification / diarization / docs as usual.

Do not refuse URL-only meeting requests. Do not require a filesystem path when a URL is present for media.

Detect common URL shapes liberally:

- `https://youtu.be/…`
- `https://www.youtube.com/watch?v=…`
- `https://…` ending in media extensions (`.mp3`, `.wav`, `.m4a`, `.mp4`, …)
- Other http(s) links the user clearly intends as a recording source

If both a file and a URL appear for the **same** input, ask which one to use (XOR InputSource).

## Workflow

1. Collect **media sources** the user wants processed — local paths **and/or** URLs (folder listing, explicit paths, or pasted links).
2. **Classify inputs** (see **Filename heuristics** below; for URLs, ask the user for role when the link alone is ambiguous) into:
   - shared mix → `role: room` (alias `merged`) — `path` or `url`
   - per-speaker tracks → `role: participant` + `participant: <id>` — `path` or `url`
   - accompanying docs/materials → `role: context` — **`path` / `uri` only** (no `url`)
3. Infer **speaker gender** from names in filenames (or known participant labels) when the user’s prompt does not state it (see **Gender**). Confirm uncertain guesses before execute.
4. If a **shared mix** and **participant tracks** both exist, ask how to use the mix (**Choices UX**, numbered) — see **Mix + tracks**:
   1. Diarize on mix
   2. Align to mix without diarize (recommended if they decline diarize)
   3. Tracks only (ignore mix)
5. Confirm the assembled `inputs` + meeting model with the user (include URLs, mix mode, any `subtitles` choices).
6. Call `process_meeting` with `execute: true` only after confirmation.
7. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Filename heuristics

Apply case-insensitively to the **basename** (ignore extension) for **path** inputs. Prefer explicit user labels over heuristics. For **URL-only** inputs with no useful basename, ask the user for `role` / `participant` instead of guessing.

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

- Pass as an `inputs[]` entry with `role: context` and `path` (or `uri`) to the folder or file.
- **Never** set `url` on context inputs — Runtime rejects it.
- Ask the user for materials if they mentioned slides/docs but did not attach paths.
- Do not put PDF/DOCX contents into chat instead of `role: context`.

## Gender

When a participant file (or known name) is present and the user prompt does **not** explicitly give gender:

1. Infer `male` / `female` / leave unset from the **given name** using common language conventions (RU/EN and other languages you know).
2. Set `meeting.participants.known[].constraints.gender` only when reasonably confident.
3. If unsure, ask once; do not invent gender for ambiguous nicknames (`Alex`, `Саша`, `Женя`) without confirmation.
4. Never override an explicit gender from the user prompt.

## Mix + tracks

When both a room mix and per-speaker tracks are present, the mix is **not** only for diarization. Present a **numbered** choice:

1. **Diarize on mix** — labels who spoke when on the shared recording; match to tracks.  
   → `meeting.diarization.enabled: true` (or `auto`), `meeting.alignment.reference: timeline` (or omit / `auto`).
2. **Align to mix without diarize** — text/speakers from tracks; mix is the **timing reference** for the final meeting document (no `diarize` step).  
   → `meeting.diarization.enabled: false`, `meeting.alignment.reference: mix` (or `auto` with diarize off).
3. **Tracks only** — ignore the mix entirely.  
   → `meeting.diarization.enabled: false`, `meeting.alignment.reference: none`.

If the user declines diarize but still wants the mix used for “who/when” timing, pick **2** — do not drop the mix.

Room-only (no participant tracks): propose diarize as before; mix can also be transcribed (`purposes` defaults include `transcript` when diarization is off).

## Diarization

| Situation | Action |
|-----------|--------|
| Mix + tracks; user did not choose a mode | Offer **Mix + tracks** choices (1/2/3 above). |
| User asked for diarization / speaker labels on the mix | Mode **1**. |
| User wants mix for timing but not diarize | Mode **2** (align to mix). |
| User said ignore mix / “only tracks” | Mode **3**. |
| Only per-speaker tracks, no mix | Skip diarization; `alignment.reference` stays default. |

Default Runtime diarization policy is `auto` when unset — still ask when a mix is present so the user picks how the mix is used.

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
- Poll `get_job` every **10s** (see below). Progress advances mainly at pipeline step boundaries; a stable percent mid-step (especially during `transcribe` / diarize) is normal.

**Ballpark duration** (wall clock, local Metal / CPU; wide variance):

- Short inputs: often **a few minutes** of Job time.
- Full meetings (long audio + merge / diarize): often **several minutes to tens of minutes**.
- URL import adds download time up front (often minutes for long YouTube / video sources).
- Tell the user the estimate is rough.

**Polling:** call `get_job` every **10s** until `completed`, `failed`, or `cancelled`. Report `progress` / `phase` when present. Only escalate after several checks with no percent/`phase` change and no Runtime error.

### Results

- When the Job reaches `completed`, use `list_artifacts` with the Job `id` to discover outputs.
- Present the main meeting / transcript artifact(s) as **clickable markdown links** (`[basename](file:///abs/path)`), not bare path strings.
- Offer a **numbered** follow-up: show in chat / open file / both / done.

### Cancellation

- If the user asks to stop processing, call `cancel_job` with the Job `id`.
- Do not start a second Job when cancellation is requested.

### Failures

- If **any** pipeline step / node fails (including one participant track in a parallel meeting), the Job ends as `failed`. Do not treat a surviving sibling branch as success.
- If the Job fails, report the Runtime error from `get_job` (and the failed node id when present).
- Do not retry automatically unless the user explicitly asks.
- Do not keep polling after `failed` / `cancelled` hoping the Job will recover.
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

If a Job fails with Metal / GPU resource errors (e.g. `Failed to create metal resource: Buffer`):

- Prefer **retry** — `vd-gigaam` chunks long audio (≤20s windows) for Metal and auto-retries on CPU if a buffer alloc still fails.
- If it still fails, or for an explicit CPU path: set `device: "cpu"` on `process_meeting` / `process_audio` and re-run (same inputs). Do not switch engine just for Metal OOM.
- Do not retry the identical Metal-only setup in a loop without CPU.

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
# Single room recording from YouTube / http(s)
process_meeting inputs=[room url:https://youtu.be/…] → …
process_meeting (audio.url: https://youtu.be/…, subtitles: prefer) → …
# Room URL + local participant tracks + docs
process_meeting inputs=[room url:https://…, participant:alice.wav, context:./docs] → …
process_meeting → job_id → get_job (until completed) → list_artifacts
process_meeting → job_id → cancel_job
```

## Notes

Reply in the user's / agent's conversation language (see Language above).

URL import uses the shared Runtime resolver (`vd-input` / `vd-url`). Meeting planners see resolved local audio paths — do not inject `import-url` into Jobs yourself.
