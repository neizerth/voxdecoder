# ADR 0010 — vd-fix-asr Local Transcript Cleanup

**Status:** Implemented — all 6 stages + confidence policy + layered dictionaries shipped (see Decision)  
**Type:** ADR / architectural RFC  
**Date:** 2026-08-03

**Related:** [`vd-fix-asr`](../../src/cli/fix/vd-fix-asr/) · [`vd-fix-casing`](../../src/cli/fix/vd-fix-casing/) · [`vd-fix-terms`](../../src/cli/fix/vd-fix-terms/) · [`vd-fix-layout`](../../src/cli/fix/vd-fix-layout/) · [`vd-pipeline`](../../src/cli/process/vd-pipeline/) · [`vd-artifact`](../../src/crates/vd-artifact/) · [`vd-progress`](../../src/crates/vd-progress/) · [`vd-output`](../../src/crates/vd-output/) · [ADR 0012 — fix-disfluency / fix-overlap RFC](0012-local-cleanup-disfluency-and-overlap.md) · [ADR 0013 — vd-text shared linguistic infrastructure RFC](0013-local-linguistic-infrastructure.md)

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

**Implemented.** Shipped as a staged, incrementally-delivered rollout inside the existing `vd-fix-asr` crate (`asr/rule.rs`, `asr/stages/`, `asr/lang/`, `asr/context_fuzzy.rs`, `asr/report.rs`), landing one stage per PR in order: spacing → punctuation → duplicates → merge/split → alphabet → dictionary, followed by confidence policy / CLI surface (`--strict`, `--aggressive`, `--report`, `--dictionary`, `--project`). The legacy lexicon-based fixer (formerly `asr/backend/mod.rs`) was migrated into the Stage 6 dictionary (static lookup in `asr/stages/dictionary.rs` + context/neighbor fuzzy matching in `asr/context_fuzzy.rs`) rather than discarded. Every stage kept `vd-fix-asr run` producing valid, non-regressive output at each step — `vd-pipeline` only invokes this crate as an external binary step, so no Job contract changes were required.

Final shape:
- **Rule engine**: `asr/rule.rs` (`Rule` trait, `Confidence`, `RuleCategory`, `RuleHit`) + `asr/stages/mod.rs` (`Stage`, `Pipeline`, `ConfidencePolicy`, `RuleStage` helper for Certain-only stages).
- **Stages 1–5** (`spacing.rs`, `punctuation.rs`, `duplicate.rs`, `merge_split.rs`, `alphabet.rs`) are pure text-in/text-out, composed via `Pipeline`.
- **Stage 6** is split in two: a context-free static lookup (`stages/dictionary.rs`, fits `Pipeline`) and a `SpanContext`-dependent fuzzy pass (`context_fuzzy.rs`, runs as a separate step in `fixer.rs` since `Pipeline` is deliberately context-free).
- **Dictionary layering** (`asr/lang/mod.rs::resolve_dictionary`): `builtin → pack (reserved, unused) → project (.voxdecoder/asr-dictionary.yml) → user (--dictionary)`, built on `vd_assets::load_dictionary`.
- **`--report`** (`asr/report.rs`): per-category counts of *applied* hits plus an `unsafe` count for hits withheld by the active `ConfidencePolicy`.
- 51 unit tests + 14 e2e tests, `cargo clippy -- -D warnings` clean.

### PR 1 audit: overlap with `vd-fix-casing`

Confirmed by reading `vd-fix-casing/src/casing/backend/normalize.rs`: it already runs `collapse_ws` (all whitespace runs, including tabs/newlines, → single space), `tidy_punct_spacing` (drops space before `,.;:!?`), and `collapse_duplicate_periods` (3+ `.` → `…`) — functionally the same transforms as `vd-fix-asr`'s new Stage 1/2. Since the pipeline order is `fix-casing → fix-asr → fix-terms`, Stage 1/2 are no-ops on text that already went through `fix-casing` in a full pipeline run. This is accepted as intentional defense-in-depth, not a bug: `vd-fix-asr` is also invoked standalone (outside `vd-pipeline`), where no upstream normalization is guaranteed. Both rule sets are idempotent, so running them twice is safe. No dedup work planned — revisit only if the two implementations drift in behavior for the same input.

### Non-goals tension with current docs

`vd-fix-asr`'s current `README.md`/`STRUCTURE.md` state that punctuation, casing, and whitespace are *not* this tool's job (owned instead by `vd-fix-casing`). This ADR's stages 1–2 (spacing, punctuation) intentionally narrow, not remove, that boundary: `vd-fix-asr` now owns **character-level, in-span** whitespace/punctuation normalization (tabs, run-of-spaces, line-ending style, `...`→`…`, space-before-punctuation, duplicate punctuation) as a precondition for reliable dictionary/ASR-artifact matching. `vd-fix-casing` continues to own **case decisions**, `vd-fix-layout` continues to own **paragraph-level reflow**, `vd-fix-terms` continues to own **terminology**. `README.md`/`STRUCTURE.md`'s non-goals sections are updated to reflect this narrower boundary as the corresponding stages land (spacing/punctuation stage PR), rather than all at once here.
