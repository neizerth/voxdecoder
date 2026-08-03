# ADR 0010 — vd-fix-asr Local Transcript Cleanup (RFC)

**Status:** RFC (future — not implemented under this design yet)  
**Type:** ADR / architectural RFC  
**Date:** 2026-08-03

**Related:** [`vd-fix-asr`](../../src/cli/fix/vd-fix-asr/) · [`vd-fix-casing`](../../src/cli/fix/vd-fix-casing/) · [`vd-fix-terms`](../../src/cli/fix/vd-fix-terms/) · [`vd-fix-layout`](../../src/cli/fix/vd-fix-layout/) · [`vd-pipeline`](../../src/cli/process/vd-pipeline/) · [`vd-artifact`](../../src/crates/vd-artifact/) · [`vd-progress`](../../src/crates/vd-progress/) · [`vd-output`](../../src/crates/vd-output/)

Layout / CLI for the existing crate remain: [STRUCTURE.md](../../src/cli/fix/vd-fix-asr/STRUCTURE.md) · [cli.md](../../src/cli/fix/vd-fix-asr/cli.md) · [RUST.md](../../src/cli/fix/vd-fix-asr/RUST.md).

---

## Context

Today’s `vd-fix-asr` is a **rules backend** focused on wording repair (misheard terms, mixed scripts, etc.). This ADR captures a stronger product direction: **fully local, deterministic transcript cleanup without an LLM** — a potential differentiator for VoxDecoder.

This document is intentional architecture for a future iteration. It does **not** mandate an immediate rewrite of the current binary.

---

## Goal

Improve transcript quality **without changing its meaning**.

`vd-fix-asr` performs deterministic cleanup of typical speech recognition artifacts.

It is **not** an editor.

It is **not** an LLM.

It never rewrites text.

---

## Philosophy

```text
ASR
  ↓
Transcript
  ↓
vd-fix-asr
  ↓
Cleaner transcript
  ↓
layout
  ↓
terms
  ↓
postprocess
```

Its purpose is to repair recognition artifacts that are predictable.

---

## Core rule

```text
Only improve confidence.
Never improve writing.
```

The tool may:

- remove obvious ASR artifacts
- normalize spacing
- merge split words
- split merged words
- normalize punctuation
- normalize repeated tokens

The tool never:

- paraphrases
- rewrites
- summarizes
- translates
- invents words
- changes meaning

---

## Scope

The tool owns only deterministic cleanup.

### Word merge

```text
каккак  →  как
этотоже →  это тоже
```

### Duplicate words

```text
эти эти стандарты  →  эти стандарты
```

### Duplicate punctuation

```text
....  →  …
```

### Broken punctuation

```text
Да , конечно  →  Да, конечно
```

### Mixed alphabets

```text
SРE      →  SRE
JS Fidls →  JS Fiddle
```

Only when confidence is extremely high.

### Repeated filler syllables

```text
ииии  →  и
ээээ  →  ээ
```

### Broken words

```text
поопробуем → попробуем
френтенд   → фронтенд
```

Only if deterministic.

---

## Out of scope

Not owned by `vd-fix-asr`:

- terminology normalization → `vd-fix-terms`
- capitalization → `vd-fix-casing`
- paragraph layout → `vd-fix-layout`
- diarization
- punctuation restoration (generative)
- grammar correction
- style improvements
- summarization

---

## Architecture

Instead of one opaque model: a **staged pipeline**. Each stage performs exactly one class of cleanup.

| Stage | Class | Examples |
|-------|--------|----------|
| 1 | Whitespace | tabs, multiple spaces, line endings |
| 2 | Punctuation | `...` → `…`, space before comma, duplicate punctuation |
| 3 | Duplicates | `каккак` → `как`, `вот вот` → `вот` |
| 4 | Merge / split | dictionary-assisted (`этотоже` → `это тоже`) |
| 5 | Alphabet | Latin / Cyrillic / mixed → canonical form |
| 6 | ASR dictionary | frequent recognition mistakes |

### ASR dictionary (example)

```yaml
JS Fidls:
  replace: JSFiddle
RBNB:
  replace: Airbnb
Avisales:
  replace: Aviasales
```

Project dictionaries may extend this.

---

## Confidence

Every replacement has confidence:

```text
certain | likely | unsafe
```

Unsafe replacements are never applied automatically.

---

## Rule engine

Every rule belongs to one category:

```text
spacing | punctuation | duplicate | merge | split | dictionary | alphabet
```

Rules are composable.

---

## Dictionaries

Layered dictionaries:

```text
builtin → language pack → project → user
```

Project dictionary example:

```yaml
Aviasales
SRE
GraphQL
TypeScript
```

---

## Language packs

Initial support: `ru`, `en`.

Each pack provides:

- common ASR errors
- merge rules
- split rules
- filler tokens
- punctuation heuristics

---

## Artifact guarantees

Never changes:

- timestamps
- speaker labels
- ids
- metadata

May change only transcript text.

---

## CLI (target shape)

```bash
vd-fix-asr run \
    -i meeting.json \
    --language ru
```

Future options:

```bash
--dictionary terms.yml
--project .voxdecoder
--strict
--aggressive
--report
```

### Optional report

```json
{
  "spacing": 42,
  "duplicates": 18,
  "merged": 6,
  "split": 3,
  "dictionary": 11,
  "unsafe": 2
}
```

---

## Pipeline placement

Recommended order:

```text
transcribe
  ↓
fix-casing
  ↓
fix-asr
  ↓
fix-layout
  ↓
fix-terms
  ↓
postprocess
```

`fix-asr` should run **before** terminology normalization: many canonical terms cannot be matched until obvious ASR artifacts are repaired.

---

## Future directions (still deterministic-first)

Optional quality improvements via pluggable analyzers — without making generative rewrite the default:

- confidence-aware cleanup using token confidences from ASR engines
- phonetic correction from recognition lattices / confusion sets
- language-model scoring for competing **deterministic** fixes
- optional local neural corrector as a **verification** stage, not a rewriter

Default behavior of `vd-fix-asr` must remain fully local, reproducible, and free of generative rewriting.

---

## Consequences

- Captures a clear product bet: local deterministic cleanup as a VoxDecoder strength.
- Separates this RFC from today’s narrower rules implementation; migration can be incremental (stage-by-stage) without breaking Job contracts.
- Constrains future PRs: no LLM paraphrase path in the default fix-asr capability.

---

## Decision

**Accepted as future direction (RFC).** Implementation is deferred. Current `vd-fix-asr` remains the shipping rules backend until this design is deliberately adopted.
