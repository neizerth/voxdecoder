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
7. **Artifact output location** (see section below) — **required** absolute `working_dir` / `output.dir` = conversation project folder. **Refuse** `execute: true` if unset or if it would land in the VoxDecoder checkout while the chat project is elsewhere.
8. **Prior run / leftovers** (see section below) — if intermediates or meeting artifacts already exist next to the media **or** in the chosen output dir, ask overwrite vs continue **before** execute.
9. Call `process_meeting` with `execute: true` only after confirmation. When using the default, **omit** `speed` (do not pass `2.0`). Pass `overwrite: true` only when the user chose a fresh reprocess. Always pass absolute `working_dir` (and `output.dir` when set).
10. Follow the **Runtime Contract** below for status, artifacts, cancellation, failures, and recovery.

## Artifact output location

Final `meeting_*.json` / `meeting_*.md` and cleanup siblings (`*.clean.md`) must land in the **conversation project root** — the folder the user selected for this Claude / Cursor / Claude Code project (the project the chat is attached to), **not** wherever media lives and **not** a random code checkout the agent is editing.

| Rule | Detail |
|------|--------|
| **Required** | Always pass absolute `working_dir` (prefer same for `output.dir`). Omitting it makes Runtime use `.` = **vdctl workspace** (often the VoxDecoder source tree) — that is a bug in the agent call, not an acceptable default for end-user chats. |
| How to resolve | Claude Code / Cursor: the opened project folder (e.g. `~/Downloads` if that is the project). Prefer `pwd` / project root of **this chat**, never `vdctl.toml` `workspace` unless the user opened that repo as the project. |
| Media elsewhere | Keep `inputs[].path` as absolute paths to the real files. Do **not** copy media into the project unless the user asks. |
| Forbidden | Writing meeting/cleanup results into the VoxDecoder source tree (or any other repo) unless that tree **is** the selected conversation project. If artifacts appear there by mistake → move/rewrite into the project folder and fix the next Job’s `working_dir`. |
| Cleanup | Sibling next to the primary `meeting_….md` (same directory Runtime wrote). |
| Claude files | If the host has no usable project folder, use the client’s usual user-files / attachments area the product already uses for generated docs — still never the VoxDecoder checkout by accident. |

When media directory ≠ conversation project root, present **Choices UX** once before execute:

1. **Conversation project folder** (default) → `working_dir` / `output.dir` = project root
2. **Next to media inputs** → `working_dir` / `output.dir` = media folder

Always show the chosen absolute output path in the pre-execute confirmation.

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
- For **cleanup strategies**, use AskUserQuestion **`multiSelect: true`** (see **Conservative transcript cleanup**). Next appears only after **≥1** option is selected. To decline cleanup / optional style, use the client **Skip** control — do **not** add a Skip / None option inside the question.

## Filename heuristics

Apply case-insensitively to the **basename** (ignore extension) for **path** inputs. Prefer explicit user labels over heuristics. For **URL-only** inputs with no useful basename, ask the user for `role` / `participant` instead of guessing.

### Shared mix (`role: room` / `merged`)

Treat as the common room recording when the name contains tokens such as:

`mix`, `mixed`, `merged`, `all`, `room`, `full`, `combined`, `common`, `overall`, `together`, `весь`, `общ`, `микс`, `слит`, `полный`

Examples: `meeting_mix.wav`, `all.mp4`, `merged_track.m4a`, `общая_запись.wav`.

### Per-speaker (`role: participant`)

If the basename looks like a **person name** (or contains one) and is **not** a mix token:

- Set `role: participant`.
- Set `participant` to a stable id. Prefer the **original script** from the filename (`Игорь`, `Мария_Смирнова`). An ASCII slug (`igor`) is OK only when the source name is already Latin — **never transliterate Cyrillic → Latin** for `participant` or display.
- Put display `name` under `meeting.participants.known` using the **same original script and casing** as the person’s name in the filename / user prompt (`Игорь`, not `Igor`).

```text
Filename / label     participant id     known[].name
Игорь.wav            Игорь              Игорь
Владимир.m4a         Владимир           Владимир
alice.wav            alice              Alice
```

Artifact **filenames** may still use ASCII when the OS/tooling needs it; that is separate. **Speaker labels inside `meeting.md` / turns must not be Latinized.**

Examples: `Alice.wav`, `ivan-petrov.mp3`, `Мария_Смирнова.m4a`, `Игорь.wav`.

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
- Prefer the human-readable **`meeting_YYYY-MM-DD_<participants>.md`** (speaker header only when the speaker changes; consecutive same-speaker turns are blank-line paragraphs under one `**Name**`) as the main deliverable; also keep the matching `.json` for machine use.
- Present artifact(s) as **clickable markdown links** (`[basename](file:///abs/path)`), not bare path strings — lead with the dated `meeting_….md` when present.
- After linking the primary artifact, **immediately** offer the cleanup strategy multiSelect — see **Conservative transcript cleanup**. Do **not** insert a separate menu (show in chat / open file / both / cleanup / done) before that offer.
- Never clean up automatically; user declines via the client **Skip** control (no Skip item in the list).
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
- Do **not** put **Skip** / **None of these** in the option list — Claude already has a **Skip** button. Client Skip on Q1 = no cleanup; on Q2 = no optional style strategies.
- Max **4 options** per question — split if needed.

**Question 1** (`multiSelect: true`):

```text
The transcript is ready: [basename](file:///…)

Which cleanup strategies? Select at least one (required for Next).
Defaults = all Recommended. Client Skip = leave transcript as-is.

• Fix obvious ASR mistakes (Recommended)
• Normalize technical terminology (Recommended)
• Remove noise / эканье-аканье / husks + normalize formatting (Recommended)
```

(Label 3 combines noise + stutter syllables + husks + formatting so the three Recommended defaults fit in one question.)

**Question 2** (`multiSelect: true`) — only when Q1 was answered (not client-Skipped):

```text
Optional style strategies? Select at least one (required for Next), or Skip for none.

• Make spoken language more natural
• Remove filler words (типа / как бы / mid-sentence discourse)
```

- Client **Skip** on Q1 → apply nothing.
- Otherwise apply exactly what was selected; label 3 enables noise **and** formatting — and **must** strip trailing redundant `Угу`/`Ага`, collapse echo invites (`Ну давай. Давай, давай.` → `Ну давай.`), clear empty discourse husks, **and** strip эканье/аканье/stutter runs (see table). Leaving `… HS. Угу. Угу.` / `Давай, давай.` echo residue / `А В.` / `Во, да-да-да-да-да. Хмм.` after Recommended noise = incomplete pass. Sole-turn `Угу.` / `Ага.` **keep**.
- Client **Skip** on Q2 → no style strategies.
- **Normalize technical terminology** — only highly certain names; never guess; use glossary/`docs` when available.
- **Remove filler words** = mid-sentence discourse fillers inside otherwise real speech (`типа`, `как бы`…). **≠** Recommended noise (syllable garbage, orphan letters, **эканье/аканье**, stutter `да-да-да`, echo `давай`/`ладно` runs, **trailing** `Угу`/`Ага`, empty husks). Sole-turn backchannel acks are not noise.
- **Make spoken language more natural** — only when explicitly selected.

### Default strategies (recommended bundle)

| Strategy | Scope |
|----------|--------|
| Fix obvious ASR mistakes | duplicates, merged/split words, punctuation, whitespace, obvious spelling |
| Normalize technical terminology | highly certain tech names only |
| Remove obvious speech-recognition noise | syllable garbage (`ээээ`, `мммм`, `а-а-а`, `э-э-э`); **эканье/аканье** and stutter runs (`да-да-да-да`, `нет-нет-нет`); **echo invitation repeats** (`Ну давай. Давай, давай.` → `Ну давай.`; also `ладно`/`хорошо`/… same pattern); searching/empty particles (`Во.`, `Хмм.`, `А-`) when not carrying content; orphan / glued letter junk (`А В.`, stray Latin crumbs); **trailing / redundant** `Угу` / `Ага` / `Мгм` after substantive content (`… HS. Угу. Угу.` → `… HS.`); **empty discourse husks** with no propositional content (`Вот.` / `Кайф.` / `Вот, наверное, как-то так. Угу. Кайф.` / searching `Во, да-да-да… Хмм.`) — clear or strip junk (keep turn boundary). **Keep** sole-turn meaningful acks (`**Владимир**` + `Угу.`). Do **not** strip mid-sentence `ну`/`вот` from substantive turns here unless the token is pure stutter |
| Normalize formatting | whitespace, repeated punct, obvious mixed-script *token* junk (e.g. `SРE`→`SRE`) — **never** change speaker labels / person names’ script or casing |

### Optional strategies

| Strategy | Scope |
|----------|--------|
| Make spoken language more natural | light conversational simplification |
| Remove filler words | mid-sentence discourse fillers in substantive turns (`типа`, `как бы`, `в общем`, `короче`, filler `наверное` / `как-то так`) — keep meaning; do not gut real hedges the speaker needed |

### Preserve

Never change: meaning, facts, chronology, technical content, **speaker attribution** (**`**Name**` lines must stay byte-for-byte** — do not Latinize, do not retitle everyone `Игорь`, do not drop `Владимир` / other speakers), timestamps, uncertainty, document structure (including speaker turn boundaries) — except text edits explicitly allowed by a selected strategy.

**Speaker-label hard rule (cleanup):**

1. Copy each `**Speaker**` header from the source transcript **byte-for-byte** (script + casing). Cleanup edits **body text only**.
2. Do **not** merge, split, reorder, or reassign turns.
3. **Never invent pipeline / role ids as speakers.** Forbidden labels unless they already appear as `**…**` headers in the source: `room`, `merged`, `mix`, `track-0`, `SPEAKER_00`, `S0`, `S1`, branch ids, filenames. If the source has `**Игорь**` / `**Владимир**`, those stay — do **not** replace either with `**room**`.
4. After writing `.clean.md`, verify:
   - every `**Name**` in clean ∈ source speaker-label set;
   - no new labels appeared (especially not `room`);
   - sole-turn `Угу.` kept when it was a sole-turn ack.
   If cleanup introduced `room` / dropped a real name / collapsed everyone to one speaker → **discard** that clean file and redo (body-only edits).

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

If the user accepts cleanup, write the cleaned text to a **new** sibling file next to the primary artifact in the **same output directory** (conversation project root — see **Artifact output location**), e.g. `meeting_….clean.md`. Overwrite only when they explicitly ask to replace. Prefer a new file by default. Keep `meeting.json` unchanged unless the user asks to update it too. Briefly list which strategies were applied. Never drop `.clean.md` into the VoxDecoder source tree by accident.

### Execution — max quality, min tokens/ops

When the user opts in (any selected strategy), run cleanup **once, tightly**:

1. **One read** of the primary transcript artifact (prefer `.md` / `.txt` already linked — do not re-fetch via Runtime).
2. **One write** of the cleaned sibling (or overwrite if explicitly requested). No draft files, no intermediate copies.
3. **Single model pass** over the whole transcript for all selected strategies together — do not run one strategy per turn, do not re-clean the output.
4. **Do not paste** the transcript (or large excerpts) into chat. Edit via tools; chat gets only: path to cleaned file + short list of applied strategies (+ optional short uncertain-fixes note).
5. **Chunk only if required** (context limits). If chunking: contiguous speaker-turn boundaries, non-overlapping chunks, same strategy set, stitch in order — still one write at the end. Prefer not chunking when the file fits.
6. **No extra tool churn:** no `get_job` / `list_artifacts` / re-plan / shell loops for cleanup; no second AskUserQuestion mid-pass; no “thinking out loud” per fix.
7. **Quality inside that pass:** apply every selected strategy thoroughly; keep the conservative rule (when in doubt, leave text). Uncertain candidates → at most a **short** bullet list in chat, not a second rewrite.

Anti-patterns (forbidden once cleanup is agreed):

- Streaming play-by-play of each correction
- Multiple rewrite rounds “to be sure”
- Re-reading the artifact after writing unless the write failed
- Summarizing the meeting as a side effect of cleanup

Goal: **highest-confidence corrections allowed by the selected strategies, in the fewest tool calls and output tokens.**

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
