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
4. Continue with docs / confirmation as usual (default: **omit** `speed` = 1×).

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
   - **Default: 1× (no speedup)** → **omit** `speed` unless the user picks otherwise.
   - Always show a **numbered choice list** with the default marked (see **Choices UX**):
     1. **1× / no speedup** (default) → omit `speed`
     2. **1.5×** → `speed: 1.5`
     3. **2.0×** → `speed: 2.0`
     4. **2.2×** → `speed: 2.2`
   - Prefer 1× for quality; speedup trades accuracy for wall time.
4. **Prior run / leftovers** — if `*.prepared.*` / `*.fixed.txt` (or similar) already exist next to the media (or under `.voxdecoder/work/`), ask before execute (**Choices UX**):
   1. **Overwrite / reprocess from scratch** → `overwrite: true` (default when the user asked to re-run / «заново» / after bad quality)
   2. **Continue / reuse existing intermediates** → omit `overwrite` or `overwrite: false`
   Same choice if a Job fails with `output already exists` / `AlreadyExists`.
5. Confirm run with a short summary + numbered **Start / Cancel** (or equivalent). Call `process_audio` with `execute: true` only after they confirm. When using the default, **omit** `speed`. Pass `overwrite: true` only when chosen. On macOS you do not need to set `device` — the Runtime defaults to Metal.
6. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Choices UX

Whenever the user must pick among options (docs, speed, overwrite vs continue, confirm run, post-result actions):

- Present **numbered options** (1 / 2 / 3 …), one line each — so the client can render selectable choices.
- Mark the default explicitly, e.g. `1. 1× / no speedup (default)`.
- Do **not** bury options inside a single prose paragraph (“ok, or diff (1.5/2.2/none)?”).
- One question block at a time when possible; avoid stacking unrelated free-form prompts.
- For **cleanup strategies**, use AskUserQuestion **`multiSelect: true`** (see **Conservative transcript cleanup**). Next appears only after **≥1** option is selected — always include a **Skip cleanup** option so the user can proceed without enabling strategies.

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
- When reporting status to the user, include **all** of these from `get_job` when present: `progress`, `phase`, `processed`, `total`, `unit` (e.g. `18% · transcribing · 3/12 chunk`). Do not omit `processed`/`total`/`unit` from the user-facing status line.
- Do not use HTTP polling (`curl`, `/health`, `/jobs/…`).
- Do not inspect Runtime sockets directly.

**Ballpark duration** (wall clock, local Metal / CPU; wide variance):

- Short audio (≈1–5 min media): often **1–5 minutes** of Job time.
- Longer media / video / URL import: often **several minutes to tens of minutes** (URL resolve downloads first).
- Preprocess `speed` (1.5× / 2.0× / 2.2×) shortens ASR wall time; tell the user the estimate is rough.

**Polling interval (adaptive — never spam):**

- **Forbidden:** polling every 10s / 15s / 30s. Floor is **60s**; default is **90s**.
- Compute next wait from the latest `get_job`:
  - If `unit` is `chunk` and `total` is set:  
    `remaining = total - processed` (treat missing `processed` as 0)  
    `interval_sec = clamp(90, 300, remaining * 45)`  
    (≈45s per remaining ASR chunk; long audio → 2–5 min between polls.)
  - If counters are absent (early load / non-ASR step): **90s**.
  - Near the end (`progress` ≥ 90 or only fix steps left): **60–90s**.
- Prefer MCP `get_job` + agent wakeup/schedule for that interval. Do **not** write shell `until`/`while` poll loops. If you must use CLI once, `vdctl api job.status … --json` — and **never** assign a shell variable named `status` (readonly in zsh); use `job_status` / `st`.

**Liveness (do not treat as stuck):**

- Overall `progress` may stay at **0%** (or flat) for a long time while a single step runs (especially `transcribe`).
- If `processed`/`total` (or `phase`) **moves** between polls, the Job is alive — keep waiting; do not escalate, cancel, or restart.
- Only escalate after **several** checks (each ≥ the adaptive interval above) where **none** of `progress`, `phase`, `processed`, or `total` changed and there is no Runtime error.

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
- After linking the primary artifact, **immediately** offer the cleanup strategy multiSelect (or Skip) — see **Conservative transcript cleanup**. Do **not** insert a separate menu (show in chat / open file / both / cleanup / done) before that offer.
- Never clean up automatically; skip is always available as a selectable option.
- If the user later asks to see the transcript in chat, read the file and paste it (truncate with a note only if extremely long).

### Cancellation

- If the user asks to stop processing, call `cancel_job` with the Job `id`.
- Do not start a second Job when cancellation is requested.

### Failures

- If the Job fails, report the Runtime error from `get_job`.
- If the error looks like an artifact conflict (`output already exists`, `AlreadyExists`, «output exists»), offer overwrite vs continue (Choices UX) and retry once with the user’s pick — do not loop.
- Do not retry automatically for other failures unless the user explicitly asks.
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

## Conservative transcript cleanup

Until deterministic local `vd-fix-asr` ([ADR 0010](../../docs/adr/0010-vd-fix-asr-local-transcript-cleanup.md)) owns primary cleanup, this Skill may offer an **optional** AI cleanup pass. Full policy: [ADR 0011](../../docs/adr/0011-conservative-ai-transcript-cleanup-in-skills.md).

**Opt-in only.** Never run cleanup unless the user explicitly agrees. Never edit outside selected strategies.

### Strategy offer (multiSelect)

Right after the artifact link (Results), present cleanup — **not** as a later menu item.

**Claude / AskUserQuestion:**

- Use **`multiSelect: true`**. It works; **Next appears only after ≥1 option is selected**.
- Boxes start **unchecked** — markdown `[x]` does not pre-select. In the question text, tell the user: for defaults, select every option marked Recommended.
- **Always** include **Skip cleanup** (or **None of these** on optional-only questions) so the user can unlock Next without enabling strategies.
- Max **4 options** per question — split if needed.

**Question 1** (`multiSelect: true`):

```text
The transcript is ready: [basename](file:///…)

Which cleanup strategies? Select at least one (required for Next).
Defaults = all Recommended. Skip = leave transcript as-is.

• Fix obvious ASR mistakes (Recommended)
• Normalize technical terminology (Recommended)
• Remove noise + normalize formatting (Recommended)
• Skip cleanup
```

(Label 3 combines noise + formatting so all four safe defaults fit with Skip in one 4-option question.)

**Question 2** (`multiSelect: true`) — only when Q1 is not Skip:

```text
Optional style strategies? Select at least one (required for Next).

• Make spoken language more natural
• Remove filler words
• None of these
```

- **Skip cleanup** → apply nothing (ignore other Q1 selections if mixed).
- Otherwise apply exactly what was selected; label 3 enables both noise and formatting.
- **None of these** → no style strategies.
- **Normalize technical terminology** — only highly certain names; never guess; use glossary/`docs` when available.
- **Remove filler words** (`как бы`, `типа`…) ≠ noise (`ээээ`, `мммм`).
- **Make spoken language more natural** — only when explicitly selected.

### Default strategies (recommended bundle)

| Strategy | Scope |
|----------|--------|
| Fix obvious ASR mistakes | duplicates, merged/split words, punctuation, whitespace, obvious spelling |
| Normalize technical terminology | highly certain tech names only |
| Remove obvious speech-recognition noise | syllable garbage / recognition junk without meaning |
| Normalize formatting | whitespace, repeated punct, Cyrillic/Latin mixups — no wording change |

### Optional strategies

| Strategy | Scope |
|----------|--------|
| Make spoken language more natural | light conversational simplification |
| Remove filler words | discourse fillers that may still matter for analysis |

### Preserve

Never change: meaning, facts, chronology, technical content, speaker attribution, timestamps, uncertainty, document structure — except edits explicitly allowed by a selected strategy.

### Forbidden (regardless of strategies)

Never: summarize, paraphrase, rewrite, improve style (unless **Make spoken language more natural** is selected), reorder sentences, merge/split speaker turns, translate, infer missing information, remove meaningful repetitions, invent terminology.

The transcript must remain a transcript.

### Conservative rule

```text
When in doubt: do not modify the transcript.
```

False negative ≫ false correction.

### Transparency

Uncertain fixes — mention, do not apply silently:

```text
Possible correction:

JS Fidls
↓
JSFiddle

Not applied automatically because confidence is insufficient.
```

If the user accepts cleanup, write the cleaned text to a **new** sibling file (e.g. `*.clean.md` / `*.clean.txt`) or overwrite only when they explicitly ask to replace the artifact. Prefer a new file by default. Briefly list which strategies were applied.

## Examples

```text
process_audio (path: /work/meeting.wav) → job_id → get_job → list_artifacts
process_audio (url: https://youtu.be/…) → job_id → …
process_audio (url: https://youtu.be/…, subtitles: prefer) → …
process_audio (speed: 1.5|2.0|2.2) → job_id → get_job → list_artifacts
process_audio (docs: /path/to/materials) → …
process_audio (video.mp4) → preprocess extract-audio → …
process_audio → job_id → cancel_job
```

## Notes

Reply in the user's / agent's conversation language (see Language above).

URL import uses the shared Runtime resolver (`vd-input` / `vd-url`). Planners see only resolved audio artifacts — do not inject `import-url` into Jobs yourself.
