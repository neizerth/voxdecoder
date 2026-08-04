# ADR 0011 — Optional AI Transcript Cleanup Strategies

**Status:** Accepted (skills behavior)  
**Type:** ADR  
**Date:** 2026-08-03

**Related:**

- [`skills/vd-audio`](../../skills/vd-audio/)
- [`skills/vd-meeting`](../../skills/vd-meeting/)
- Runtime Contract in Skills
- [ADR 0010 — vd-fix-asr local cleanup RFC](0010-vd-fix-asr-local-transcript-cleanup.md)
- Future: `vd-fix-asr`, `vd-fix-layout`, `vd-fix-terms`

---

## Motivation

ASR output is often already usable but contains minor recognition artifacts.

Until deterministic local cleanup (`vd-fix-asr`) is available, Skills may offer an optional AI-assisted cleanup.

The cleanup must remain conservative and predictable.

Rather than asking:

> "Do you want me to clean the transcript?"

the Skill should let the user explicitly choose which classes of modifications are allowed.

---

## Goal

Introduce optional cleanup strategies.

Each strategy grants permission for a specific class of edits.

The AI must never perform edits outside the selected strategies.

---

## Decision

`vd-audio` and `vd-meeting` Skills **must** offer an **optional**, **opt-in** transcript cleanup pass after a successful Job. Cleanup is never automatic. The user picks strategies via AskUserQuestion **`multiSelect: true`** (Next requires ≥1 selection). Decline via the client **Skip** control — do **not** put **Skip cleanup** / **None of these** in the option list. Style-changing strategies stay off unless explicitly selected. Wording and semantics must be identical across transcript-producing Skills.

---

## User flow

After transcription completes, link the artifact and **immediately** offer cleanup strategies. Do **not** insert an intermediate menu (show in chat / open file / both / cleanup / done) before this offer.

**Claude / AskUserQuestion:** `multiSelect: true` works. **Next appears only after ≥1 option is selected.** Boxes are not pre-checked (`[x]` in markdown does nothing). Do **not** offer **Skip cleanup** / **None of these** as list items — Claude already has a **Skip** button (Q1 Skip = no cleanup; Q2 Skip = no optional style). Max 4 options per question.

**Question 1** (`multiSelect: true`):

```text
The transcript is ready: [basename](file:///…)

Which cleanup strategies? Select at least one (required for Next).
Defaults = all Recommended. Client Skip = leave transcript as-is.

• Fix obvious ASR mistakes (Recommended)
• Normalize technical terminology (Recommended)
• Remove noise / эканье-аканье / husks + normalize formatting (Recommended)
```

**Question 2** (`multiSelect: true`) — if Q1 was answered (not client-Skipped):

```text
Optional style strategies? Select at least one (required for Next), or Skip for none.

• Make spoken language more natural
• Remove filler words (типа / как бы / mid-sentence discourse)
```

Only apply strategies the user selected (client Skip on Q1 → nothing).

---

## Default strategies

The following strategies are enabled by default.

### Fix obvious ASR mistakes

Repairs only obvious speech-recognition errors.

Examples:

- duplicated words
- merged words
- split words
- duplicated punctuation
- whitespace
- obvious spelling artifacts

```text
этотоже  →  это тоже
каккак   →  как
```

### Normalize technical terminology

Correct technical names only when the correction is highly certain.

Examples:

- product names
- APIs
- package names
- frameworks
- programming languages
- companies

Never guess. Unknown terms remain unchanged.

### Remove obvious speech-recognition noise

Remove recognition artifacts that clearly do not carry meaning.

Examples:

```text
ээээ
мммм
ииии
э-э-э
а-а-а
да-да-да-да
А В.
Во.
Хмм.
```

Repeated filler syllables, **эканье/аканье**, stutter affirmation runs, recognition garbage, accidental repeated fragments, orphan / glued letter junk, searching/empty particles without content.

**Backchannels (`Угу` / `Ага` / `Мгм`):**

- **Strip** trailing / redundant acks after substantive content in the same turn:
  ```text
  На предпоследней строчке. Ну, у тебя там HS. Угу. Угу.
  →
  На предпоследней строчке. Ну, у тебя там HS.
  ```
- **Keep** when the turn’s entire text is a sole meaningful ack (one word or a short backchannel-only reply):
  ```text
  **Владимир**
  Угу.
  ```
- Local `vd-fix-disfluency` applies the same trailing-strip / sole-keep rule deterministically (ADR 0014).

**Echo invitation / encouragement repeats** (not hyphen stutter `да-да-да`):

```text
Ну давай. Давай, давай. Ну, пример. Но, при reduceRight у тебя
→
Ну давай. Ну, пример. Но, при reduceRight у тебя
```

Collapse adjacent same-word echoes of short invites (`давай`, `ладно`, `хорошо`, …) across comma/period — keep one. Do **not** gut real emphasis that adds meaning when uncertain. Local CLI does the allowlisted deterministic form.

Also clear **empty discourse husks** with no propositional content (not sole meaningful acks), e.g.:

```text
Вот.
Кайф.
Вот, наверное, как-то так. Угу. Угу. Кайф.
Во, да-да-да-да-да. Где она? То у нас. Вот, наверное. Хмм.
```

Keep turn / paragraph boundaries (do not merge or split speaker turns). Emptying such husks is allowed; mid-sentence `ну` / `вот` inside otherwise real speech stays unless **Remove filler words** is selected (pure stutter runs may still be stripped under Recommended noise). Do **not** clear a sole-turn `Угу.` / `Ага.` — that is a real conversational response.

Natural speech should be preserved whenever uncertain.

### Normalize formatting

Normalize presentation without changing wording.

Examples:

- whitespace
- repeated punctuation
- obvious mixed-script *token* junk (`SРE` → `SRE`)
- obvious formatting inconsistencies

**Never** change speaker labels or person-name script/casing (do not Latinize `Игорь` → `Igor`; do not collapse every turn to one speaker).

```text
Да , конечно  →  Да, конечно
SРE           →  SRE
```

---

## Optional strategies

Disabled by default.

### Make spoken language more natural

May lightly simplify conversational speech.

```text
Ну, в общем...  →  Да...
```

This changes speaking style and therefore requires explicit consent.

### Remove filler words

Examples:

```text
как бы
типа
в общем
короче
```

These may be meaningful in conversational analysis. Disabled by default.

Use for **mid-sentence** discourse fillers inside otherwise substantive speech. Distinct from **Remove obvious speech-recognition noise**, which covers syllable garbage (`ээээ`), **эканье/аканье**, stutter `да-да-да`, orphan letters, **trailing redundant** `Угу`/`Ага` after real content, and **empty discourse husks** (`Кайф.` / searching `Во, да-да-да… Хмм.`) — those are in the Recommended bundle. Sole-turn `Угу.` / `Ага.` stays.

---

## Forbidden operations

Regardless of selected strategies, the AI must never:

- summarize
- paraphrase
- rewrite
- improve style (except under **Make spoken language more natural**, when selected)
- reorder sentences
- merge speaker turns
- split speaker turns
- reassign or rename speakers / rewrite `**Name**` headers (including collapsing everyone to one label)
- introduce pipeline / input-role ids as speaker labels (`room`, `merged`, `mix`, `track-0`, `S0`, …) when they were not already `**…**` headers in the source — e.g. never replace `**Игорь**` with `**room**`
- translate
- infer missing information
- remove meaningful repetitions that are not noise/stutter under a selected strategy
- invent terminology
- Latinize / transliterate speaker names or other proper names’ script (e.g. `Игорь` → `Igor`) unless the source already used that script

The transcript must remain a transcript.

---

## Conservative rule

When uncertain:

```text
Preserve the original transcript.
```

False negatives are preferred over false corrections.

---

## Transparency

If a correction is uncertain:

- do not silently apply it;
- instead mention the possible correction separately.

```text
Possible correction:

JS Fidls

↓

JSFiddle

Not applied automatically because confidence is insufficient.
```

---

## Output

If the user accepts cleanup, write cleaned text to a **new** sibling file (e.g. `*.clean.md` / `meeting_….clean.md`) by default, or overwrite only when explicitly asked. Briefly list which strategies were applied. Keep structured artifacts (e.g. `meeting.json`) unchanged unless the user asks to update them too.

**Location:** sibling next to the primary transcript in the **conversation project root** the user selected for this Claude / Cursor / Claude Code session. Skills **must** pass absolute `working_dir` / `output_dir` / `output.dir` — omitting them writes into Runtime `.` (= vdctl workspace). Do not drop cleaned files into the VoxDecoder source checkout unless that checkout is the selected project. Media may live elsewhere; artifacts follow the project folder (or the user’s explicit “next to media” choice). After cleanup, verify speaker labels still match the source set — every clean `**Name**` must already exist in the source; **never** introduce `room` / role ids. If verification fails → discard `.clean.md` and redo.

---

## Execution — max quality, min tokens/ops

When the user opts in (any selected strategy), the agent **must** run cleanup as a tight single job:

1. **One read** of the linked primary transcript (`.md` / `.txt`). Do not re-poll Runtime for cleanup.
2. **One write** of the cleaned sibling (or overwrite if explicitly requested). No drafts / intermediate files.
3. **Single model pass** applying **all** selected strategies together — never one strategy per turn, never a second “polish” pass.
4. **Do not paste** the transcript into chat. Chat output = cleaned path + short applied-strategy list (+ optional brief uncertain-fixes note).
5. **Chunk only if context limits force it.** Chunks must follow contiguous turn/paragraph boundaries, not overlap, use the same strategy set, and stitch into one final write. Prefer no chunking when the file fits.
6. **No extra ops:** no `get_job` / `list_artifacts` / re-plan / exploratory shell for cleanup; no mid-pass AskUserQuestion; no per-fix narration.
7. **Quality:** within that one pass, apply selected strategies thoroughly under the conservative rule. Uncertain items → short chat note, not another rewrite.

Forbidden once cleanup is agreed: play-by-play correction streams, multi-round rewrites “to be sure”, re-read after successful write, summarizing the meeting/audio as a cleanup side effect.

Success metric for this section: **maximum allowed high-confidence quality with the fewest tool calls and chat tokens.**

---

## Future compatibility

These strategies intentionally mirror future local capabilities.

| Strategy | Future local implementation |
|----------|-----------------------------|
| Fix obvious ASR mistakes | `vd-fix-asr` |
| Normalize technical terminology | `vd-fix-terms` |
| Normalize formatting | `vd-fix-layout` + `vd-fix-asr` |
| Remove speech-recognition noise | `vd-fix-asr` |
| Make spoken language more natural | AI-only |
| Remove filler words | AI-only (or future optional module) |

As local capabilities mature, Skills should increasingly delegate cleanup to deterministic local tools.

When deterministic `vd-fix-asr` (ADR 0010) becomes primary:

```text
ASR → vd-fix-asr → (optional) AI cleanup
```

AI cleanup should become a final review layer rather than the primary cleanup mechanism.

---

## Runtime Contract

Transcript-producing Skills (`vd-audio`, `vd-meeting`) should expose the same cleanup strategies and semantics.

Users receive a consistent experience regardless of which Skill produced the transcript.

---

## Success criteria

- Cleanup remains optional.
- Users explicitly choose allowed cleanup strategies.
- Safe strategies are enabled by default.
- Style-changing strategies require explicit opt-in.
- The AI never exceeds the selected strategies.
- Opted-in cleanup runs as a **single tight pass** (min tools / chat tokens, max allowed quality).
- The design remains compatible with future local cleanup capabilities.
