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
   - Present as a **numbered choice list** (see **Choices UX**). Decline via client **Skip** — do not add a Skip item in the list.
   - If the user provides a folder or file of materials, pass it as `docs` on `process_audio` (absolute or Runtime-visible path).
   - If they Skip / there are no materials, omit `docs` (Runtime defaults to `.`).
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
5. Confirm run with a short summary that includes the **absolute output path**, plus numbered **Start / Cancel** (or equivalent). Call `process_audio` with `execute: true` only after they confirm. When using the default, **omit** `speed`. Pass `overwrite: true` only when chosen. Set `output_dir` to the conversation project root (see **Artifact output location**). On macOS you do not need to set `device` — the Runtime defaults to Metal.
6. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Artifact output location

Transcript artifacts and cleanup siblings (`*.clean.md` / `*.clean.txt`) must land in the **conversation project root** — the folder the user selected for this Claude / Cursor / Claude Code project — **not** a random code checkout the agent is editing.

| Rule | Detail |
|------|--------|
| **Required** | Always pass absolute `output_dir` (and `working_dir` when set). Omitting it makes Runtime use `.` = **vdctl workspace** (often the VoxDecoder source tree) — that is a bug in the agent call. |
| How to resolve | Opened project folder for **this chat** (e.g. `~/Downloads`). Never use `vdctl.toml` `workspace` unless that repo is the selected project. |
| Media elsewhere | Keep `audio.path` absolute to the real file. Do **not** copy media into the project unless asked. |
| Forbidden | Writing results into the VoxDecoder source tree unless that tree **is** the selected conversation project. If artifacts land there by mistake → move them and fix the next Job. |
| Cleanup | Sibling next to the primary transcript in the same output directory. |
| Claude files | If no usable project folder, use the client’s usual user-files / attachments area — never the VoxDecoder checkout by accident. |

When media directory ≠ conversation project root, present **Choices UX** once before execute:

1. **Conversation project folder** (default) → `output_dir` = project root
2. **Next to media** → `output_dir` = media folder

Always show the chosen absolute output path in the pre-execute confirmation.

## Choices UX

Whenever the user must pick among options (docs, speed, overwrite vs continue, confirm run, post-result actions):

- Present **numbered options** (1 / 2 / 3 …), one line each — so the client can render selectable choices.
- Mark the default explicitly, e.g. `1. 1× / no speedup (default)`.
- Do **not** bury options inside a single prose paragraph (“ok, or diff (1.5/2.2/none)?”).
- One question block at a time when possible; avoid stacking unrelated free-form prompts.
- For **cleanup strategies**, use AskUserQuestion **`multiSelect: true`** (see **Conservative transcript cleanup**). Next appears only after **≥1** option is selected. To decline cleanup / optional style, use the client **Skip** control — do **not** add a Skip / None option inside the question.

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
- After linking the primary artifact, **immediately** offer the cleanup strategy multiSelect — see **Conservative transcript cleanup**. Do **not** insert a separate menu (show in chat / open file / both / cleanup / done) before that offer.
- Never clean up automatically; user declines via the client **Skip** control (no Skip item in the list).
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
- Do **not** put **Skip** / **None of these** in the option list — Claude already has a **Skip** button. Client Skip on Q1 = no cleanup; on Q2 = no optional style strategies.
- Max **4 options** per question — split if needed.

**Question 1** (`multiSelect: true`):

```text
The transcript is ready: [basename](file:///…)

Which cleanup strategies? Select at least one (required for Next).
Pick "All recommended" for one click, or choose individually. Client Skip = leave transcript as-is.

• All recommended — applies the three below in one click (Recommended)
• Fix obvious ASR mistakes (Recommended)
• Normalize technical terminology (Recommended)
• Remove noise / эканье-аканье / husks + normalize formatting (Recommended)
```

(Label 3/4 combines noise + stutter syllables + husks + formatting so the three Recommended defaults fit in one question alongside the one-click "All recommended" option — four total, at the max per question.)

Selecting **All recommended** (alone or together with any of the other three) applies all three Recommended strategies — treat it as shorthand for ticking all three, not a fourth strategy of its own. Selecting only a subset of the individual three (without "All recommended") applies exactly that subset, unchanged from before — this option is purely a faster default path, it does not remove the ability to pick a partial set.

**Question 2** (`multiSelect: true`) — only when Q1 was answered (not client-Skipped):

```text
Optional style strategies? Select at least one (required for Next), or Skip for none.

• Make spoken language more natural
• Remove filler words (типа / как бы / mid-sentence discourse)
```

- Client **Skip** on Q1 → apply nothing.
- **All recommended** selected → treat as if the three Recommended strategies below it were all selected, whether or not the user also ticked any of them individually.
- Otherwise apply exactly what was selected; label 3/4 (noise) enables noise **and** formatting — and **must** strip trailing redundant `Угу`/`Ага`, collapse echo invites (`Ну давай. Давай, давай.` → `Ну давай.`), clear empty discourse husks, **and** strip эканье/аканье/stutter runs (see table). Leaving `… HS. Угу. Угу.` / `Давай, давай.` echo residue / `А В.` / `Во, да-да-да-да-да. Хмм.` after Recommended noise = incomplete pass. Sole-turn `Угу.` / `Ага.` **keep**.
- Client **Skip** on Q2 → no style strategies.
- **Normalize technical terminology** — only highly certain names; never guess; use glossary/`docs` when available.
- **Remove filler words** = mid-sentence discourse fillers inside otherwise real speech (`типа`, `как бы`…). **≠** Recommended noise (syllable garbage, orphan letters, **эканье/аканье**, stutter `да-да-да`, echo `давай`/`ладно` runs, **trailing** `Угу`/`Ага`, empty husks). Sole-turn backchannel acks are not noise.
- **Make spoken language more natural** — only when explicitly selected.

### Default strategies (recommended bundle)

| Strategy | Scope |
|----------|--------|
| Fix obvious ASR mistakes | duplicates, merged/split words, punctuation, whitespace, obvious spelling |
| Normalize technical terminology | highly certain tech names only |
| Remove obvious speech-recognition noise | syllable garbage (`ээээ`, `мммм`, `а-а-а`, `э-э-э`); **эканье/аканье** and stutter runs (`да-да-да-да`); **echo invitation repeats** (`Ну давай. Давай, давай.` → `Ну давай.`); searching/empty particles (`Во.`, `Хмм.`); orphan / glued letter junk (`А В.`); **trailing / redundant** `Угу` / `Ага` / `Мгм` after substantive content (`… HS. Угу. Угу.` → `… HS.`); **empty discourse husks** with no propositional content (`Вот.` / `Кайф.` / searching `Во, да-да-да… Хмм.`). **Keep** sole-turn meaningful acks (`Угу.` alone under a speaker). Do **not** strip mid-sentence `ну`/`вот` from substantive sentences here unless pure stutter |
| Normalize formatting | whitespace, repeated punct, obvious mixed-script *token* junk (e.g. `SРE`→`SRE`) — **never** change speaker labels / person names’ script or casing |

### Optional strategies

| Strategy | Scope |
|----------|--------|
| Make spoken language more natural | light conversational simplification |
| Remove filler words | mid-sentence discourse fillers in substantive speech (`типа`, `как бы`, `в общем`, `короче`, filler `наверное` / `как-то так`) |

### Preserve

Never change: meaning, facts, chronology, technical content, **speaker attribution** (labels/`**Name**` lines byte-for-byte — do not collapse everyone to one speaker), timestamps, uncertainty, document structure — except edits explicitly allowed by a selected strategy.

**Speaker-label hard rule (cleanup):**

1. Copy each `**Speaker**` header from the source transcript **byte-for-byte** (script + casing). Cleanup edits **body text only**.
2. Do **not** merge, split, reorder, or reassign turns.
3. **Never invent pipeline / role ids as speakers.** Forbidden unless already a source `**…**` header: `room`, `merged`, `mix`, `track-0`, `SPEAKER_00`, `S0`, `S1`, branch ids, filenames. Source `**Игорь**` / `**Владимир**` stay — never replace with `**room**`.
4. After writing `.clean.md`, verify every clean label ∈ source set and no new labels (esp. not `room`). Violation → **discard** clean file and redo.

### Forbidden (regardless of strategies)

Never: summarize, paraphrase, rewrite, improve style (unless **Make spoken language more natural** is selected), reorder sentences, merge/split speaker turns, **reassign or rename speakers**, **introduce `room` / role / branch-id labels**, translate, infer missing information, remove meaningful repetitions, invent terminology.

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

If the user accepts cleanup, write the cleaned text to a **new** sibling file next to the primary transcript in the **same output directory** (conversation project root — see **Artifact output location**), e.g. `*.clean.md` / `*.clean.txt`. Overwrite only when they explicitly ask to replace. Prefer a new file by default. Briefly list which strategies were applied. Never drop cleaned files into the VoxDecoder source tree by accident.

### Execution — max quality, min tokens/ops

When the user opts in (any selected strategy), run cleanup **once, tightly**:

1. **One read** of the primary transcript artifact (prefer the linked `.md` / `.txt` — do not re-fetch via Runtime).
2. **One write** of the cleaned sibling (or overwrite if explicitly requested). No draft files, no intermediate copies.
3. **Single model pass** over the whole transcript for all selected strategies together — do not run one strategy per turn, do not re-clean the output.
4. **Do not paste** the transcript (or large excerpts) into chat. Edit via tools; chat gets only: path to cleaned file + short list of applied strategies (+ optional short uncertain-fixes note).
5. **Chunk only if required** (context limits). If chunking: contiguous boundaries, non-overlapping chunks, same strategy set, stitch in order — still one write at the end. Prefer not chunking when the file fits.
6. **No extra tool churn:** no `get_job` / `list_artifacts` / re-plan / shell loops for cleanup; no second AskUserQuestion mid-pass; no “thinking out loud” per fix.
7. **Quality inside that pass:** apply every selected strategy thoroughly; keep the conservative rule (when in doubt, leave text). Uncertain candidates → at most a **short** bullet list in chat, not a second rewrite.

Anti-patterns (forbidden once cleanup is agreed):

- Streaming play-by-play of each correction
- Multiple rewrite rounds “to be sure”
- Re-reading the artifact after writing unless the write failed
- Summarizing the audio as a side effect of cleanup

Goal: **highest-confidence corrections allowed by the selected strategies, in the fewest tool calls and output tokens.**

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
