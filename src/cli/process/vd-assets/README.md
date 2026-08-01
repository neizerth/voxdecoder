# vd-assets — prepare project knowledge for vd-fix-*

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI signature: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [../../fix/README.md](../../fix/README.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented.** Workspace member `src/cli/process/vd-assets`.

## Core rule

```text
vd-assets builds reusable project assets for vd-fix-*.

Markdown is one artifact. `terms.yml` is another.
It does not rewrite transcripts.
```

`vd-assets` sits **before** the cleanup pipeline. Fix CLIs consume the assets directory — they do **not** open binary Office/PDF themselves.

## Pipeline

```text
pdf / docx / md / …

        ↓

    vd-assets

        ↓

.voxdecoder/
  md/
  terms.yml

        ↓

vd-fix-casing → vd-fix-asr → vd-fix-terms
 presentation      wording       terminology
                   ↑ default     ↑ default
                   .voxdecoder   .voxdecoder
```

## Quick start

```bash
vd-assets run -i ./docs
vd-assets run -i ./spec.pdf --ocr
vd-assets run -i ./docs --dry-run --json

vd-fix-asr   run -i meeting.txt
vd-fix-terms run -i meeting.txt
```

## Why its own binary

| Topic | vd-assets | Not here |
|-------|-----------|----------|
| Job | Prepare project knowledge | Fix transcript text |
| Input | Project files / dirs (md, pdf, docx, xlsx, …) | Transcript artifacts (`txt`/`srt`/…) |
| Output | Assets dir: `md/` + `terms.yml` | `{stem}.fixed.{ext}` |
| OCR | Optional (scanned docs) | — |
| Authority | Source documents | Typography / meaning / lexicon (fix CLIs) |

## Behavior

1. Collects `-i` files and directories.
2. If **no** Markdown exists among inputs → Office/PDF conversion is **mandatory**.
3. Converts PDF / DOCX / XLSX (legacy `.doc` via `textutil` on macOS) → `out/md/*.md`.
4. Copies existing text/Markdown into `out/md/`.
5. Extracts canonical terminology from processed text → `out/terms.yml`.

Extract cache (`$VD_ASSETS_CACHE` or platform cache): same source fingerprint + OCR mode → skip re-parse. Cache stores extracted text.

## Output layout

```text
.voxdecoder/
  md/              # Markdown (converted + copied text sources)
  terms.yml        # forms + structured entries for vd-fix-*
  env              # optional: VD_PROJECT_DIR=…
```

## Guarantees

`vd-assets` never:

- rewrites transcript artifacts
- invents glossary entries without text support in the sources
- calls cloud OCR / conversion APIs
- replaces `vd-fix-*` — it only prepares materials

## Library use

The crate exposes `vd_assets::load_dictionary` for fix CLIs. Prefer the project assets directory (`.voxdecoder`); text / Markdown / `terms.yml` also work. Passing Office/PDF into that loader errors with a hint to run this CLI first.

Full flags, progress, exit codes: [cli.md](cli.md).
