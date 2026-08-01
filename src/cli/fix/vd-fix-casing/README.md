# vd-fix-casing — presentation only

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI signature: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).  
Languages beyond `ru` / `en`: [TODO-languages.md](TODO-languages.md).

`vd-fix-casing` fixes **presentation** of text artifacts without changing words or meaning. First step in the local cleanup pipeline (`vd-fix-casing` → `vd-fix-asr` → `vd-fix-terms`).

Rewrites only presentation. The input artifact type and structure are preserved.

---

## Quick start

```bash
vd-fix-casing run -i meeting.txt
vd-fix-casing run -i meeting.txt --progress=json

vd-fix-casing install ru          # optional pack cache / override
vd-fix-casing install en
vd-fix-casing list
```

`run` works without packs (embedded lexicon). `install` is optional — same UX shape as `vd-gigaam`, useful when you want a cached or updated pack on disk.

Models dir: platform cache (`~/.cache/…` / `~/Library/Caches/…`), or `VD_FIX_CASING_MODELS_DIR` / `--download-root` / `config set download_root`.

---

## Why its own binary

Presentation is a different job from wording and terminology:

| Topic | vd-fix-casing | Not here |
|-------|----------------|----------|
| Changes | Punctuation, casing, whitespace, quotes, dashes, sentence layout | Words / sense (`vd-fix-asr`) |
| Scope | Presentation of transcript text | Dictionaries (`vd-fix-terms`) |
| Discourse context | Not used | Neighboring segments for ASR repair |
| Output type | Same as input | Never restyles into another format |
| Packs | Optional `install ru` / `en` (like `vd-gigaam`); builtin without install | Shared multi-fix engine |

Shared I/O via [`crates/`](../../../crates/) (do not fork artifact/output/progress here). No ASR / glossary flags “because another binary has them”.

---

## Behavior

Changes presentation only:

- punctuation
- casing
- whitespace
- quotes
- dashes
- sentence layout

Does not:

- repair ASR errors
- normalize terminology
- translate
- rewrite sentences
- change words

**Example**

```text
мы обсуждали кубернетис и сейфтензорс
        ↓ vd-fix-casing
Мы обсуждали кубернетис и сейфтензорс.
```

Backend (rules today; ML later) stays private. CLI only sees language packs and progress.

---

## Guarantees

`vd-fix-casing` never changes:

- words
- segment boundaries
- timestamps
- speaker labels
- ids
- metadata

Only presentation of transcript text is rewritten.

**Input type == output type** (`txt→txt`, `json→json`, `srt→srt`, …). Default output stem: `.fixed.` (same for all `vd-fix-*`).

---

## Boundaries (what vd-fix-casing is not)

| Not in vd-fix-casing | Where it lives |
|----------------------|----------------|
| Misheard words / homophones | `vd-fix-asr` |
| Canonical product / API names | `vd-fix-terms` |
| Job queue / multi-run state | `vd-srv` |
| Re-transcription | `vd-gigaam` / `vd-whisper` |

Commands (`run` / `install` / `list` / …), progress, exit codes: [cli.md](cli.md).
