# vd-assets CLI

Build reusable project assets for `vd-fix-*` from documentation and other sources.

**Status: implemented.**

Product notes: [README.md](README.md). Sibling process CLIs: [../README.md](../README.md). Fix pipeline: [../../fix/README.md](../../fix/README.md).

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-assets run` | Prepare project assets (`md/` + `terms.yml`) |
| `vd-assets config` | Default settings |

Shorthand: `vd-assets -i DIR` ≡ `vd-assets run -i DIR` (writes `.voxdecoder/` by default).

---

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

vd-fix-asr          # default --context .voxdecoder
vd-fix-terms         # default --terms .voxdecoder
```

Do **not** pass raw PDF/DOCX to `vd-fix-*`. Prepare assets here first.

### Project directory

Default assets root: **`.voxdecoder/`** (shared by all CLIs via [`vd-artifact::paths`](../../../crates/vd-artifact/)).

| Source | Effect |
|--------|--------|
| `$VD_PROJECT_DIR` | Process env override |
| `.voxdecoder/env` or `.env` | `VD_PROJECT_DIR=…` |
| nearest `.voxdecoder/` | Walk up from input / cwd |

```bash
vd-assets run -i ./docs                 # → ./.voxdecoder
vd-assets run -i ./docs -o ./custom
vd-fix-asr run -i meeting.txt            # → reads ./.voxdecoder if present
vd-fix-terms run -i meeting.txt
```

---

## Commands

### `vd-assets run`

#### Input / output

| Argument | Short | Required | Description |
|----------|-------|----------|-------------|
| `--input` | `-i` | ✅ | Source file or directory. **Repeatable** |
| `--output` | `-o` | — | Output assets directory (`md/` + `terms.yml`). Default: `.voxdecoder` (or `$VD_PROJECT_DIR`) |

**Output layout**

```text
{output}/
  md/              # converted + copied text sources
  terms.yml     # forms + structured entries (aliases, canonical, metadata)
```

Later additions (`metadata.json`, `cache/`, …) can land in the same directory without changing the CLI.

#### Behavior

1. Collect files under `-i` (recursive for directories).
2. Classify: text/Markdown vs convertible Office/PDF.
3. If **no** Markdown/text among inputs → conversion of Office/PDF is **mandatory** (error if nothing convertible).
4. Convert PDF / DOCX / XLSX / legacy DOC → `md/*.md`.
5. Copy text/Markdown sources into `md/`.
6. Extract canonical terminology from processed text → `terms.yml`.

Supported convertible types: `pdf`, `docx`, `doc` (macOS `textutil`), `xlsx` / `xlsm` / `xls`.

Text sources used without conversion: `md`, `txt`, `yaml` / `yml`, `json`, and other text/code extensions.

#### Options

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--ocr` | — | off | Enable OCR for scanned documents |
| `--force` | — | off | Ignore extract cache; re-parse sources |
| `--dry-run` | — | — | Print resolved plan and exit (no write) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` or `json` |
| `--quiet` | `-q` | — | Disable progress on stderr |

#### Examples

```bash
vd-assets run -i ./docs
vd-assets run -i ./docs -o ./.voxdecoder
vd-assets -i ./spec.pdf --ocr
vd-assets run -i ./docs -i ./glossary.yaml
vd-assets run -i ./docs --force
vd-assets run -i ./docs --dry-run
vd-assets run -i ./docs --dry-run --json
vd-assets run -i ./docs --progress=json
vd-assets run -i ./docs -q
```

##### `--dry-run`

Text (default):

```text
Inputs: ./docs
Output: ./assets
Markdown dir: ./assets/md
Terms: ./assets/terms.yml
OCR: off
Force: off
```

Machine-readable (`--dry-run --json`):

```json
{
  "inputs": ["./docs"],
  "output": "./assets",
  "ocr": false,
  "force": false,
  "markdown_dir": "./assets/md",
  "terms": "./assets/terms.yml"
}
```

#### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Processing / convert / write error |
| 2 | Invalid arguments or invalid CLI usage |
| 3 | Input path missing / unreadable; unsupported situation (e.g. conversion required but nothing convertible) |

Exit 2 includes: unknown option, `--json` without `--dry-run`.

After a successful run, stdout prints a short summary (assets path, Markdown path, terms path, converted count, forms count) unless the process fails earlier.

---

### `vd-assets config`

```bash
vd-assets config list
vd-assets config get progress
vd-assets config set progress json
vd-assets config set ocr on
vd-assets config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|-------------|
| `progress` | `text` | `text` / `json` |
| `ocr` | `off` | Default OCR preference (CLI `--ocr` still wins when set) |

Priority: CLI > config > default.

Config path: `$VD_ASSETS_CONFIG` or platform config dir for `vd-assets`.

---

## Progress

| Value | Description |
|-------|-------------|
| `text` | Human-readable progress on stderr (default) |
| `json` | NDJSON events on stderr |

Omit progress with `-q` / `--quiet`.

Shared scheme via [`vd-progress`](../../../crates/vd-progress/): `start` → `phase`* → `done` | `error`.

### `run --progress=json`

```json
{"event":"start","input":"…","output":"…","artifact_type":"assets"}
{"event":"phase","phase":"converting","percent":10}
{"event":"phase","phase":"terms","percent":80}
{"event":"done","output":"…/terms.yml","path":"…/md","duration_sec":1.2,"char_count":420}
{"event":"error","code":"convert_failed","message":"…"}
```

---

## Cache

Extract cache root: `$VD_ASSETS_CACHE` or platform cache (`vd-assets/extract/`).

Cache stores extracted text. Fingerprint = path key + size + mtime + OCR mode. `--force` rebuilds.

---

## Using assets from fix CLIs

If `.voxdecoder/` exists next to the input, fix CLIs use it automatically:

```bash
vd-assets run -i ./docs
vd-fix-asr   run -i meeting.txt
vd-fix-terms run -i meeting.txt
```

Override with `--context` / `--terms`, `$VD_PROJECT_DIR`, or `VD_PROJECT_DIR=` in `.voxdecoder/env` / `.env`. Do **not** pass raw PDF/DOCX.

---

## Public contract note

Exact Office/PDF extractor internals and OCR engine are **implementation details** outside the public CLI contract. The CLI exposes inputs, assets output dir, optional OCR, force, dry-run, and progress.
