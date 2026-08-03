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
5. Preprocess **speed** (shorter ASR wall time; timestamps remapped via TimeMap):
   - **Default: 1× (no speedup)** → **omit** `speed` unless the user picks otherwise.
   - Always show a **numbered choice list** with the default marked (see **Choices UX**):
     1. **1× / no speedup** (default) → omit `speed`
     2. **1.5×** → `speed: 1.5`
     3. **2.0×** → `speed: 2.0`
     4. **2.2×** → `speed: 2.2`
   - Prefer 1× for quality (punctuation, rare words, meeting dialogue). Speedup trades quality for wall time.
6. Confirm the assembled `inputs` + meeting model + speed with the user (include URLs, mix mode, any `subtitles` choices).
7. **Prior run / leftovers** (see section below) — if intermediates or meeting artifacts already exist next to the media, ask overwrite vs continue **before** execute.
8. Call `process_meeting` with `execute: true` only after confirmation. When using the default, **omit** `speed` (do not pass `2.0`). Pass `overwrite: true` only when the user chose a fresh reprocess.
9. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Prior run / leftovers

Skills do **not** silently wipe work dirs. Before `execute: true`, check for existing intermediates next to each media input (same folder and `.voxdecoder/work/`):

- `*.prepared.mp3` / `*.prepared.wav` / `*.prepared.txt` / `*.prepared.fixed.txt`
- `*.timemap.json` / `*.segments.json`
- prior `meeting_*.json` / `meeting_*.md` (or legacy `meeting.json` / `meeting.md`)

If any of these exist, present **Choices UX** (do not bury in prose):

1. **Overwrite / reprocess from scratch** (default when the user asked to «перезапусти», «заново», «без старого 2×», after bad quality) → `overwrite: true`
2. **Continue / reuse existing intermediates** (default when resuming a cancelled/partial run and quality was fine) → omit `overwrite` or `overwrite: false`

Also offer this choice when `get_job` fails with `output already exists` / `AlreadyExists` / «output exists» — then retry with the user’s pick.

Never invent a third option like «delete work dir by hand» unless the user asks.

## Choices UX

Whenever the user must pick among options (mix mode, speed, overwrite vs continue, confirm run, post-result actions):

- Present **numbered options** (1 / 2 / 3 …), one line each — so the client can render selectable choices.
- Mark the default explicitly, e.g. `1. 1× / no speedup (default)`.
- Do **not** bury options inside a single prose paragraph (“ok, or 1.5/2.2/none?”).
- One question block at a time when possible; avoid stacking unrelated free-form prompts.
- For **cleanup strategies**, use AskUserQuestion **`multiSelect: true`** (see **Conservative transcript cleanup**). Next appears only after **≥1** option is selected — always include a **Skip cleanup** option so the user can proceed without enabling strategies.

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

Documents and non-media materials for **vd-assets** (glossaries, agendas, attendee lists, PDFs, markdown) — these feed **fix-asr / fix-terms**:

- Prefer `inputs[].role: context` with `path` (or `uri`), **or** top-level MCP `docs: /path/to/folder-or-file` (same effect).
- **Never** set `url` on context inputs — Runtime rejects it.
- If the user pastes accompanying notes in chat (no file path): write them to a local markdown file next to the media (e.g. `./.voxdecoder/context/notes.md`) and pass that path as `docs` / `role: context`. Do **not** leave notes only in chat.
- Ask the user for materials if they mentioned slides/docs but did not attach paths.
- Do not put PDF/DOCX binary contents into chat instead of `role: context`.
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
| `context` | `inputs[].role: context` **or** MCP `docs` | `prepare-context` → vd-assets → terms/names for fixers |

## Runtime Contract

This Skill starts long-running Runtime Jobs.

### Execution

- Start with `process_meeting`.
- The tool returns a Job `id` (also called `job_id`).

### Progress

- Use `get_job` with that `id` to monitor execution until `completed`, `failed`, or `cancelled`.
- When reporting status to the user, include **all** of these from `get_job` when present: `progress`, `phase`, `processed`, `total`, `unit` (e.g. `18% · transcribing · 3/12 chunk`). Do not omit `processed`/`total`/`unit` from the user-facing status line.
- Do not use HTTP polling (`curl`, `/health`, `/jobs/…`).
- Do not inspect Runtime sockets directly.

**Ballpark duration** (wall clock, local Metal / CPU; wide variance):

- Short inputs: often **a few minutes** of Job time.
- Full meetings (long audio + merge / diarize): often **several minutes to tens of minutes**.
- URL import adds download time up front (often minutes for long YouTube / video sources).
- Preprocess `speed` (1.5× / 2.0× / 2.2×) shortens ASR wall time; tell the user the estimate is rough.

**Polling interval (adaptive — never spam):**

- **Forbidden:** polling every 10s / 15s / 30s. Floor is **60s**; default is **90s**.
- Compute next wait from the latest `get_job`:
  - If `unit` is `chunk` and `total` is set:  
    `remaining = total - processed` (treat missing `processed` as 0)  
    `interval_sec = clamp(90, 300, remaining * 45)`  
    (≈45s per remaining ASR chunk; long meetings → 2–5 min between polls.)
  - If counters are absent (early load / non-ASR step): **90s**.
  - Near the end (`progress` ≥ 90 or only fix/merge nodes left): **60–90s**.
- Prefer MCP `get_job` + agent wakeup/schedule for that interval. Do **not** write shell `until`/`while` poll loops. If you must use CLI once, `vdctl api job.status … --json` — and **never** assign a shell variable named `status` (readonly in zsh); use `job_status` / `st`.

**Liveness (do not treat as stuck):**

- Overall `progress` may stay at **0%** (or flat) for a long time while a single step runs (especially `transcribe`).
- If `processed`/`total` (or `phase`) **moves** between polls, the Job is alive — keep waiting; do not escalate, cancel, or restart.
- Only escalate after **several** checks (each ≥ the adaptive interval above) where **none** of `progress`, `phase`, `processed`, or `total` changed and there is no Runtime error.

### Results

- When the Job reaches `completed`, use `list_artifacts` with the Job `id` to discover outputs.
- Prefer the human-readable **`meeting_YYYY-MM-DD_<participants>.md`** (speaker blocks: bold name on its own line, text on the next; blank line between turns) as the main deliverable; also keep the matching `.json` for machine use.
- Present artifact(s) as **clickable markdown links** (`[basename](file:///abs/path)`), not bare path strings — lead with the dated `meeting_….md` when present.
- After linking the primary artifact, **immediately** offer the cleanup strategy multiSelect (or Skip) — see **Conservative transcript cleanup**. Do **not** insert a separate menu (show in chat / open file / both / cleanup / done) before that offer.
- Never clean up automatically; skip is always available as a selectable option.
- If the user later asks to see the transcript in chat, read the file and paste it (truncate with a note only if extremely long).

### Cancellation

- If the user asks to stop processing, call `cancel_job` with the Job `id`.
- Do not start a second Job when cancellation is requested.

### Failures

- If **any** pipeline step / node fails (including one participant track in a parallel meeting), the Job ends as `failed`. Do not treat a surviving sibling branch as success.
- If the Job fails, report the Runtime error from `get_job` (and the failed node id when present).
- If the error looks like an artifact conflict (`output already exists`, `AlreadyExists`, «output exists»), offer **Prior run / leftovers** choices and retry once with the user’s pick — do not loop.
- Do not retry automatically for other failures unless the user explicitly asks.
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

## Conservative transcript cleanup

Until deterministic local `vd-fix-asr` ([ADR 0010](../../docs/adr/0010-vd-fix-asr-local-transcript-cleanup.md)) owns primary cleanup, this Skill may offer an **optional** AI cleanup pass. Full policy: [ADR 0011](../../docs/adr/0011-conservative-ai-transcript-cleanup-in-skills.md).

**Opt-in only.** Never run cleanup unless the user explicitly agrees. Never edit outside selected strategies. Never merge or split speaker turns while cleaning.

### Strategy offer (multiSelect)

Right after the artifact link (Results), present cleanup — **not** as a later menu item. Never merge or split speaker turns while cleaning.

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

Never change: meaning, facts, chronology, technical content, speaker attribution, timestamps, uncertainty, document structure (including speaker turn boundaries) — except edits explicitly allowed by a selected strategy.

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

If the user accepts cleanup, write the cleaned text to a **new** sibling file (e.g. `meeting_….clean.md`) or overwrite only when they explicitly ask to replace the artifact. Prefer a new file by default. Keep `meeting.json` unchanged unless the user asks to update it too. Briefly list which strategies were applied.

## Examples

```text
# Mix + speakers + docs (default: omit speed = 1×)
process_meeting inputs=[room:mix.wav, participant:alice.wav, context:./docs] → …
# Video mix → extract-audio inside preprocess
process_meeting inputs=[room:meeting.mp4] + diarization → …
# Single room recording from YouTube / http(s)
process_meeting inputs=[room url:https://youtu.be/…] → …
process_meeting (audio.url: https://youtu.be/…, subtitles: prefer) → …
# Room URL + local participant tracks + docs
process_meeting inputs=[room url:https://…, participant:alice.wav, context:./docs] → …
# Optional speedup (quality tradeoff): 1.5 | 2.0 | 2.2
process_meeting (speed: 1.5|2.0|2.2) → …
process_meeting → job_id → get_job (until completed) → list_artifacts
process_meeting → job_id → cancel_job
```

## Notes

Reply in the user's / agent's conversation language (see Language above).

URL import uses the shared Runtime resolver (`vd-input` / `vd-url`). Meeting planners see resolved local audio paths — do not inject `import-url` into Jobs yourself.
