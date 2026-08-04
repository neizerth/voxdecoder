# ADR 0014 — Orphan Letters and Filler Cleanup

**Status:** Partially implemented — glued-onset wired via `vd-text` → `vd-fix-disfluency`; orphan/hyphen-stutter detect remains in `vd-text` (full CLI apply path for orphans still expanding)  
**Type:** ADR / architectural RFC  
**Date:** 2026-08-03

**Related:**

- [`vd-fix-disfluency`](../../src/cli/fix/vd-fix-disfluency/) · [ADR 0012](0012-local-cleanup-disfluency-and-overlap.md)
- [`vd-fix-asr`](../../src/cli/fix/vd-fix-asr/) · [ADR 0010](0010-vd-fix-asr-local-transcript-cleanup.md)
- [`vd-text`](../../src/lib/vd-text/) · [ADR 0013](0013-local-linguistic-infrastructure.md)
- [`vd-pipeline`](../../src/cli/process/vd-pipeline/)

---

## Motivation

Even after high-quality ASR, transcripts frequently contain small speech artifacts that significantly reduce readability.

Typical examples:

- orphan letters
- repeated syllables
- hesitation sounds
- speech fillers
- false starts

These artifacts are highly deterministic and can be removed without language models.

---

## Goal

Extend `vd-fix-disfluency` with deterministic cleanup of orphan letters and filler fragments.

The objective is **not** to rewrite spoken language.

The objective is to remove artifacts that are almost certainly produced by spontaneous speech or ASR.

---

## Core rule

```text
Remove speech artifacts.

Preserve speech.
```

Never change wording unless the removed fragment carries no semantic value.

---

## Scope

Owns:

- orphan letters
- glued onset stutter (`Ччисто` → `Чисто`)
- hesitation syllables
- repeated syllables
- false starts
- repeated fillers

Does not own:

- duplicate phrases
- paragraph layout
- terminology
- ASR spelling
- overlap removal

---

## Artifact classes

### 1. Orphan letters

Single letters that are not meaningful words.

Examples:

```text
я... э... думаю

↓

я думаю
```

```text
м... хорошо

↓

хорошо
```

```text
а... ну...

↓

ну...
```

Only when clearly acting as hesitation.

### 2. Hesitation sounds

Examples:

```text
ээ
эээ
эм
мм
ммм
```

↓

removed

or

↓

collapsed

depending on configuration.

### 3. Glued onset stutter

A leading letter (often from a truncated false start) glued onto the next word with no hyphen or space. Common after a cut segment ends mid-word (`чи…` + `чисто` → `Ччисто`).

Examples:

```text
Ччисто никаких ошибок

↓

Чисто никаких ошибок
```

```text
Ддавай

↓

Давай
```

Rule (deterministic, high confidence):

- Word length ≥ 4.
- First character equals the second (case-insensitive), and the remainder is a plausible word form (or the full token without the first char matches a known continuation).
- Do **not** apply to intentional initials, acronyms, or protected tokens.

Owned by `vd-fix-disfluency` via `vd-text` (same binary as orphan / hyphen stutter). Distinct from ASR dictionary repairs (`идимо`→`видимо` → `vd-fix-asr`).

### 4. Stuttering

Repeated syllables.

Examples:

```text
я-я думаю

↓

я думаю
```

```text
по-поэтому

↓

поэтому
```

```text
н-н-ну

↓

ну
```

```text
и-и-и

↓

и
```

### 5. Repeated fillers

Examples:

```text
ну... ну...

↓

ну...
```

```text
так-так

↓

так
```

```text
да-да-да

↓

да
```

Only when clearly involuntary.

### 6. Empty hesitation chains

Examples:

```text
ээ...
эм...
мм...

↓

removed
```

---

## Conservative rules

Never remove:

```text
Ну да.
Ну конечно.
Да-да.
Так-так.
Вот именно.
Ага.
Угу.
```

These may carry conversational meaning.

Only remove when classified as hesitation.

---

## Detection signals

The implementation should combine multiple signals.

### Lexical

Known filler dictionary.

Examples:

```text
ээ
эм
мм
ну
вот
```

### Morphological

Using Natasha:

- interjections
- particles
- incomplete tokens

### Structural

- repeated syllables
- repeated letters
- isolated one-letter tokens
- interruption markers

### Context

Examples:

Sentence beginning:

```text
ээ...
```

↓

very likely removable.

Between words:

```text
я э думаю
```

↓

likely removable.

Standalone utterance (sole-turn ack):

```text
угу
```

↓

preserve.

Trailing redundant backchannels after substantive content (same span):

```text
… там HS. Угу. Угу.
```

↓

```text
… там HS.
```

Deterministic in `vd-fix-disfluency` (light+): strip trailing `угу` / `ага` / `мгм` only when the span also has non-backchannel words. Sole-turn / backchannel-only spans stay.

Echo invitation repeats (allowlisted short invites: `давай`, `ладно`, `хорошо`, …):

```text
Ну давай. Давай, давай. Ну, пример.
```

↓

```text
Ну давай. Ну, пример.
```

Deterministic in `vd-fix-disfluency` (light+). Not general word-dedup (`Это баг. Баг.` stays).

---

## Modes

### off

No cleanup.

### light (default)

Remove only high-confidence artifacts.

Examples:

- эээ
- ммм
- н-н-ну
- и-и-и

### normal

Also remove repeated fillers.

Examples:

```text
ну... ну...
```

### aggressive

Maximum cleanup.

Includes:

- false starts
- repeated filler chains
- longer hesitation fragments

May alter speaking style.

Not recommended by default.

---

## Dictionaries

Language-specific filler dictionaries.

Example:

```yaml
ru:
  fillers:
    - э
    - ээ
    - эээ
    - эм
    - мм
    - ммм
    - ну
    - вот
    - типа
    - как бы
```

Future:

```yaml
en:
  fillers:
    - uh
    - um
    - erm
    - like
    - you know
```

---

## Pipeline position

Recommended order:

```text
ASR
↓
fix-asr
↓
fix-disfluency
↓
fix-terms
↓
fix-layout
```

Disfluency cleanup should occur before paragraph layout, reducing noise that may affect boundary detection.

---

## Reporting

Optional cleanup report:

```json
{
  "orphan_letters_removed": 12,
  "fillers_removed": 31,
  "stutters_collapsed": 18,
  "false_starts_removed": 7,
  "preserved_uncertain": 5
}
```

This improves debugging and regression testing.

---

## Shared implementation

The logic should live in `vd-text`.

Suggested modules:

```text
vd-text/
    disfluency/
        fillers.rs
        stutter.rs
        orphan_letters.rs
        detector.rs
        dictionary.rs
```

`vd-fix-disfluency` becomes a thin orchestration layer over these reusable components.

---

## Success criteria

- High-confidence hesitation artifacts are removed deterministically.
- Meaningful conversational markers are preserved.
- Cleanup is configurable (`off`, `light`, `normal`, `aggressive`).
- Language-specific dictionaries are supported.
- The implementation is reusable across all transcript-processing pipelines.
- No language model is required.
