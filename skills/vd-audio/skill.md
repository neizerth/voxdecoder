# Audio processing

## Purpose

Plan and run audio processing Jobs on the VoxDecoder Runtime via MCP (`process_audio` / `plan.audio`).

Prefer MCP tools over inventing file paths. Ask for confirmation before `execute: true`.

The default audio Job includes layout fixing (`fix-layout`) after casing / ASR / terms — readable paragraphs without changing lexical content.

Video **files** are fine: preprocess extracts the audio track with **ffmpeg** (`extract-audio`) and uses that WAV as the Job source.

**Online URLs** (YouTube, direct media links, …) are also first-class. Pass them as `audio.url`. The Runtime resolves the URL into local artifacts **before** Job planning (ADR 0008 / `vd-input`). You do **not** invent download steps or call `vd-url` yourself.

**Language:** This Skill is written in English. Reply to the user in their language (or the agent's configured conversation language). Do not switch user-facing messages to English just because this document is English.

## Input recognition

Accept any of these as the media source (exactly one):

| User gives | MCP `audio` field |
|------------|-------------------|
| Local audio / video file | `path: "/abs/or/runtime/path"` |
| `file://…` | `uri: "file://…"` |
| YouTube / http(s) media URL | `url: "https://…"` |
| Prior Runtime artifact id | `artifact: "…"` |

### When there is no file path

If the user pastes only a link (YouTube, youtu.be, direct `.mp3` / `.mp4` URL, …) and **no** local file:

1. Treat it as `audio.url` — do **not** ask them to download the file first.
2. Confirm the URL with the user.
3. Optionally ask about subtitles for YouTube-like sources: `ignore` (default) · `prefer` · `require` → MCP field `subtitles`.
4. Continue with docs / confirmation as usual (`speed: 2` by default).

Do not refuse URL-only requests. Do not require a filesystem path when a URL is present.

Detect common URL shapes liberally:

- `https://youtu.be/…`
- `https://www.youtube.com/watch?v=…`
- `https://…` ending in media extensions (`.mp3`, `.wav`, `.m4a`, `.mp4`, …)
- Other http(s) links the user clearly intends as the recording source

If both a file and a URL appear, ask which one to use (XOR InputSource).

## Workflow

1. Confirm the **media** input with the user — **path or URL** (audio or video file, or online link).
2. Ask about **accompanying documents / materials** (agendas, glossaries, name lists, PDFs, markdown, slides notes). These feed `vd-assets` via `prepare-context` and improve `fix-asr` / `fix-terms`.
   - Present as a **numbered choice list** (see **Choices UX**). Include **Skip** as an option.
   - If the user provides a folder or file of materials, pass it as `docs` on `process_audio` (absolute or Runtime-visible path).
   - If they skip / there are no materials, omit `docs` (Runtime defaults to `.`).
   - Do not dump document contents into the chat as a substitute for `docs` — point the Runtime at the files.
3. Preprocess speed (shorter ASR wall time; timestamps remapped via TimeMap):
   - **Default: 2×** → pass `speed: 2` unless the user picks otherwise.
   - Always show a **numbered choice list** with the default marked (see **Choices UX**):
     1. **2×** (default) → `speed: 2`
     2. **1.5×** → `speed: 1.5`
     3. **2.2×** → `speed: 2.2`
     4. **No speedup** → omit `speed`
4. Confirm run with a short summary + numbered **Start / Cancel** (or equivalent). Call `process_audio` with `execute: true` only after they confirm. Always include `speed: 2` when using the default. On macOS you do not need to set `device` — the Runtime defaults to Metal.
5. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Choices UX

Whenever the user must pick among options (docs, speed, confirm run, post-result actions):

- Present **numbered options** (1 / 2 / 3 …), one line each — so the client can render selectable choices.
- Mark the default explicitly, e.g. `1. 2× (default)`.
- Do **not** bury options inside a single prose paragraph (“ok, or diff (1.5/2.2/none)?”).
- One question block at a time when possible; avoid stacking unrelated free-form prompts.

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
- Poll `get_job` every **10s** (see below). Progress advances mainly at pipeline step boundaries; a stable percent mid-step (especially during `transcribe`) is normal.

**Ballpark duration** (wall clock, local Metal / CPU; wide variance):

- Short audio (≈1–5 min media): often **1–5 minutes** of Job time.
- Longer media / video / URL import: often **several minutes to tens of minutes** (URL resolve downloads first).
- Preprocess `speed` (1.5× / 2× / 2.2×) shortens ASR wall time; tell the user the estimate is rough.

**Polling:** call `get_job` every **10s** until `completed`, `failed`, or `cancelled`. Report `progress` / `phase` when present. Only escalate after several checks with no percent/`phase` change and no Runtime error.

### Results

- When the Job reaches `completed`, use `list_artifacts` with the Job `id` to discover outputs.
- Prefer the final fixed transcript when present (name often ends with `.fixed.txt` / `.fixed.md`); otherwise the best available transcript artifact.
- **Always** present artifact paths as a **clickable markdown link**, not a bare path string:

  ```markdown
  [audio.prepared.fixed.txt](file:///Users/…/audio.prepared.fixed.txt)
  ```

  - Link text = **basename** only.
  - `href` = `file://` + absolute path (percent-encode spaces).
  - Do **not** paste a long absolute path as plain monospace / prose without a link.
- After linking the primary artifact, offer a **numbered** follow-up (Choices UX), for example:
  1. Show transcript in chat
  2. Open / reveal the file (user clicks the link)
  3. Both
  4. Done
- If they pick “show in chat”, read the file and paste the transcript (truncate with a note only if extremely long).

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

If a Job fails with Metal / GPU resource errors (e.g. `Failed to create metal resource: Buffer`):

- Prefer **retry** — `vd-gigaam` chunks long audio (≤20s windows) for Metal and auto-retries on CPU if a buffer alloc still fails.
- If it still fails, or for an explicit CPU path: set `device: "cpu"` on `process_audio` and re-run. Do not switch engine just for Metal OOM.
- Do not retry the identical Metal-only setup in a loop without CPU.

If a Job is already running / the Runtime answers:

- Use Runtime tools (`get_job`, `cancel_job`, `list_artifacts`).
- Never poll Runtime over HTTP.
- Do not suggest lifecycle commands.

## Examples

```text
process_audio (path: /work/meeting.wav, speed: 2) → job_id → get_job → list_artifacts
process_audio (url: https://youtu.be/…, speed: 2) → job_id → …
process_audio (url: https://youtu.be/…, subtitles: prefer, speed: 2) → …
process_audio (speed: 1.5|2|2.2) → job_id → get_job → list_artifacts
process_audio (docs: /path/to/materials, speed: 2) → …
process_audio (video.mp4, speed: 2) → preprocess extract-audio → …
process_audio → job_id → cancel_job
```

## Notes

Reply in the user's / agent's conversation language (see Language above).

URL import uses the shared Runtime resolver (`vd-input` / `vd-url`). Planners see only resolved audio artifacts — do not inject `import-url` into Jobs yourself.
