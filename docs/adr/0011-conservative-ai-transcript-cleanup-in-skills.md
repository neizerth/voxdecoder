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

`vd-audio` and `vd-meeting` Skills **must** offer an **optional**, **opt-in** transcript cleanup pass after a successful Job. Cleanup is never automatic. The user picks strategies via AskUserQuestion **`multiSelect: true`** (Next requires ≥1 selection — always include **Skip cleanup** / **None of these**). Style-changing strategies stay off unless explicitly selected. Wording and semantics must be identical across transcript-producing Skills.

---

## User flow

After transcription completes, link the artifact and **immediately** offer cleanup strategies. Do **not** insert an intermediate menu (show in chat / open file / both / cleanup / done) before this offer.

**Claude / AskUserQuestion:** `multiSelect: true` works. **Next appears only after ≥1 option is selected.** Boxes are not pre-checked (`[x]` in markdown does nothing). Always offer **Skip cleanup** (or **None of these**) so the user can proceed without enabling strategies. Max 4 options per question.

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

**Question 2** (`multiSelect: true`) — if not Skip:

```text
Optional style strategies? Select at least one (required for Next).

• Make spoken language more natural
• Remove filler words
• None of these
```

Only apply strategies the user selected (Skip → nothing).

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
```

Repeated filler syllables, recognition garbage, accidental repeated fragments.

Natural speech should be preserved whenever uncertain.

### Normalize formatting

Normalize presentation without changing wording.

Examples:

- whitespace
- repeated punctuation
- Cyrillic/Latin mixups
- obvious formatting inconsistencies

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

Distinct from **Remove obvious speech-recognition noise** (syllable garbage like `ээээ`), which is on by default.

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
- translate
- infer missing information
- remove meaningful repetitions
- invent terminology

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
- The design remains compatible with future local cleanup capabilities.
