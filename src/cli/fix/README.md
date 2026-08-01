# Text cleaning CLIs

Local post-processing for transcripts and other text artifacts. Three tools, almost no overlap, one natural pipeline:

```text
vd-fix-casing  →  vd-fix-asr  →  vd-fix-terms
   (form)           (words)          (terminology)
```

| CLI | Changes | Spec |
|-----|---------|------|
| `vd-fix-casing` | Presentation only | [vd-fix-casing/](vd-fix-casing/) ([cli](vd-fix-casing/cli.md)) |
| `vd-fix-asr` | Words / meaning | [vd-fix-asr/](vd-fix-asr/) ([cli](vd-fix-asr/cli.md)) |
| `vd-fix-terms` | Canonical terminology | [vd-fix-terms/](vd-fix-terms/) ([cli](vd-fix-terms/cli.md)) |

Project assets (default `.voxdecoder/` with `md/` + `terms.yml`) for `--context` / `--terms`: [`vd-assets`](../process/vd-assets/) ([cli](../process/vd-assets/cli.md)). Override via `$VD_PROJECT_DIR` or `VD_PROJECT_DIR=` in `.voxdecoder/env` / `.env`.

Queue / background runs: [`vd-srv`](../vd-srv/).

---

## Shared contract

All three CLIs share the same I/O contract so they can be chained in any order (recommended order above):

- Accept **any text artifact**: `txt`, `json`, `jsonl`, `srt`, `vtt`, `md`, and `vd-*` native artifacts.
- **Input type == output type** (`txt→txt`, `json→json`, `srt→srt`, …).
- Default output: `{stem}.fixed.{ext}` (never `.cased.` / `.clean.`).
- Shared UX: `run` / `config`, `--dry-run`, `--progress=json`, `--language`, priority CLI > config > default.
- When a CLI can use downloadable assets: **`install` / `remove` / `list` / `info`** (same shape as `vd-gigaam`). Packs are **optional** if processing works with a builtin/default — do not force `install` before `run`.

`vd-fix-casing` already supports optional language packs (`install ru` / `en`) and runs without them. Other fix CLIs should follow the same pattern when they grow models or dictionaries.

Each binary documents an explicit **Guarantees** section (what it never changes). That contract is more important than the option list: it makes the pipeline safe to chain.

How they differ is only **Behavior** — each has one core rule:

| CLI | Behavior | Core rule |
|-----|----------|-----------|
| `vd-fix-casing` | changes presentation only | Never changes words |
| `vd-fix-asr` | changes words only | Changes words only to restore meaning |
| `vd-fix-terms` | changes canonical terminology only | Never guesses |

---

## Shared crates (Rust)

Workspace libs live under [`../../crates/`](../../crates/) (`src/crates/`, not under `src/cli/fix/`).

| Crate | Path | Use when developing |
|-------|------|---------------------|
| **`vd-artifact`** | [`vd-artifact`](../../crates/vd-artifact/) | Artifact load/walk/write, shared types, `paths` helpers |
| **`vd-output`** | [`vd-output`](../../crates/vd-output/) | `-o` / `-d` / `--in-place`; caller-supplied naming (`.fixed.` for fix CLIs) |
| **`vd-progress`** | [`vd-progress`](../../crates/vd-progress/) | Stderr progress (`start` / `phase` / `done` / `error`) |

Backends (`casing/` / `asr/` / `terms/`), pack install UX, clap, and config stay **per binary**.

Overview: [src/crates/README.md](../../crates/README.md). Each CLI’s STRUCTURE has a **Shared crates?** section.

```bash
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-fix-casing
cargo test -p vd-fix-asr
cargo test -p vd-fix-terms
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

Works from dictionaries and rules — it does not “guess.”

**Fixes**

- product names
- libraries
- APIs
- companies
- project names
- abbreviations
- English identifiers

**Examples**

```text
кубернетис   →  Kubernetes
си плюс плюс →  C++
чат джипити  →  ChatGPT
рест апи     →  REST API
```

**Sources**

- `terms.yml` / `terms.json`
- Markdown / README / docs
- user glossary

**Example (end of pipeline)**

```text
Мы обсуждали кубернетес и сейф тензорс.
        ↓ vd-fix-terms
Мы обсуждали Kubernetes и SafeTensors.
```

---

## Why this split

Each tool owns one layer of the text:

1. **Form** — make it readable (`vd-fix-casing`)
2. **Sense** — fix what was misheard (`vd-fix-asr`)
3. **Terminology** — lock vocabulary to the project dictionary (`vd-fix-terms`)

Together: **presentation → meaning → terminology**. That covers almost all local transcript cleanup while keeping each binary small, clear, and independent.
