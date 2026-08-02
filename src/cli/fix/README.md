# Text cleaning CLIs

Local post-processing for long-form text artifacts. Four tools, almost no overlap, one natural pipeline:

```text
vd-fix-casing  →  vd-fix-asr  →  vd-fix-terms  →  vd-fix-layout
   (form)           (words)          (terminology)       (layout)
```

Relative to recipes:

```text
transcribe → fix-* → fix-layout → postprocess
```

`vd-fix-layout` also runs on long-form outputs **after** `vd-postprocess` (e.g. `summary.md`).

| CLI | Changes | Spec |
|-----|---------|------|
| `vd-fix-casing` | Presentation only | [vd-fix-casing/](vd-fix-casing/) ([cli](vd-fix-casing/cli.md)) |
| `vd-fix-asr` | Words / meaning | [vd-fix-asr/](vd-fix-asr/) ([cli](vd-fix-asr/cli.md)) |
| `vd-fix-terms` | Canonical terminology | [vd-fix-terms/](vd-fix-terms/) ([cli](vd-fix-terms/cli.md)) |
| `vd-fix-layout` | Layout / paragraphs (v1) | [vd-fix-layout/](vd-fix-layout/) ([cli](vd-fix-layout/cli.md)) |

Project assets (default `.voxdecoder/` with `md/` + `terms.yml`) for `--context` / `--terms`: [`vd-assets`](../process/vd-assets/) ([cli](../process/vd-assets/cli.md)). Override via `$VD_PROJECT_DIR` or `VD_PROJECT_DIR=` in `.voxdecoder/env` / `.env`.

Queue / background runs: [`vd-srv`](../vd-srv/).

---

## Shared contract

All `vd-fix-*` CLIs share the same I/O contract so they can be chained:

- Accept **any text artifact**: `txt`, `json`, `jsonl`, `srt`, `vtt`, `md`, and `vd-*` native artifacts.
- **Input type == output type** (`txt→txt`, `json→json`, `srt→srt`, …).
- Default output: `{stem}.fixed.{ext}`.
- Shared UX: `run` / `config`, `--dry-run`, `--progress=json`, `--language`, priority CLI > config > default.
- Optional packs: **`install` / `remove` / `list` / `info`** (same shape as `vd-gigaam`). Do not force `install` before `run` when a builtin exists.

Each binary documents an explicit **Guarantees** section. That contract is more important than the option list.

| CLI | Behavior | Core rule |
|-----|----------|-----------|
| `vd-fix-casing` | changes presentation only | Never changes words |
| `vd-fix-asr` | changes words only | Changes words only to restore meaning |
| `vd-fix-terms` | changes canonical terminology only | Never guesses |
| `vd-fix-layout` | changes layout only | **Never changes lexical content** |

---

## Shared crates (Rust)

| Crate | Path | Use when developing |
|-------|------|---------------------|
| **`vd-artifact`** | [`vd-artifact`](../../crates/vd-artifact/) | Artifact load/walk/write, shared types, `paths` helpers |
| **`vd-output`** | [`vd-output`](../../crates/vd-output/) | `-o` / `-d` / `--in-place`; `.fixed.` naming |
| **`vd-progress`** | [`vd-progress`](../../crates/vd-progress/) | Stderr progress |

Backends stay **per binary** (`casing/` / `asr/` / `terms/` / `layout/`).

```bash
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-fix-casing
cargo test -p vd-fix-asr
cargo test -p vd-fix-terms
cargo test -p vd-fix-layout
```

---

## `vd-fix-asr`

Fixes speech-recognition errors. Spec: [vd-fix-asr/](vd-fix-asr/) ([cli](vd-fix-asr/cli.md)).

**Rewrites only wording needed to restore meaning.**

**Priority language:** Russian with English insertions (`--language ru`).

**Uses (when available):** local LM (private), neighboring segments, glossary / dictionaries, `--context` project materials.

**Does not:** restyle (`vd-fix-casing`), force canonical terms (`vd-fix-terms`), invent unsupported content.

```text
мы используем гитхап экшенс
        ↓ vd-fix-asr
мы используем гитхаб экшенс
        ↓ vd-fix-terms
мы используем GitHub Actions
```

---

## `vd-fix-terms`

Normalizes **canonical terminology** to a single form. Spec: [vd-fix-terms/](vd-fix-terms/) ([cli](vd-fix-terms/cli.md)).

**Status: implemented** (shipping lexicon + `--terms`; optional packs later).

**Core rule:** never guesses — every replacement must be backed by shipping lexicon, `--terms`, or an explicit rule.

---

## `vd-fix-layout`

Applies **layout** (v1: paragraph breaks) to readable long-form text. Spec: [vd-fix-layout/](vd-fix-layout/) ([cli](vd-fix-layout/cli.md)).

**Status: docs / planned.**

**Primary guarantee:**

```text
Never changes lexical content.
```

Only whitespace / paragraph boundaries may change. Optional TimeMap structural hints; fully usable without TimeMap. `--language ru` \| `en` \| `auto`.

```text
v1 implements paragraph layout only.
Future layout transformations keep the same guarantee.
```

```text
… сплошной поток предложений …
        ↓ vd-fix-layout --language auto
… абзац 1 …

… абзац 2 …
```

Also:

```text
summary.md
        ↓ vd-fix-layout
summary with readable paragraphs
```

---

## Why this split

1. **Form** — `vd-fix-casing`
2. **Sense** — `vd-fix-asr`
3. **Terminology** — `vd-fix-terms`
4. **Layout** — `vd-fix-layout`

Together: **presentation → meaning → terminology → layout**, then optional **postprocess** recipes — while each binary stays small and independent.
