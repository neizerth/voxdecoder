# vd-fix-terms CLI

Local terminology normalizer (product / library / API / protocol / format names → one canonical form).

**Rewrites only wording needed to lock terms to a canonical form.** The input artifact type and structure are preserved.

**Status: implemented.**

**Language default: `ru`** — Russian with English insertions (see [TODO-languages.md](TODO-languages.md)).

It does **not** guess. Canonical forms come only from the shipping lexicon, `--terms` sources, and optional future packs.

Product notes: [README.md](README.md). Shared UX with other `vd-fix-*` CLIs. Background jobs: `vd-srv`.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-fix-terms run` | Lock terminology to canonical forms in a local text artifact |
| `vd-fix-terms config` | Default settings |

Shorthand: `vd-fix-terms -i FILE` ≡ `vd-fix-terms run -i FILE`.

### Possible future commands

`install` / `remove` / `list` / `info` **may** appear later for optional downloadable term packs — same shape as `vd-gigaam` / `vd-fix-casing`, and only if processing cannot stay fully shipping-lexicon + `--terms`. Do not rely on them in product docs yet.

---

## Commands

### `vd-fix-terms run`

#### Input / output

**Input type == output type.**

```text
txt   → txt
json  → json
jsonl → jsonl
srt   → srt
vtt   → vtt
md    → md
```

Also accepts `vd-*` native artifacts; type is preserved.

| Argument | Short | Required | Description |
|----------|-------|----------|-------------|
| `--input` | `-i` | ✅ | Path to a text artifact |
| `--output` | `-o` | — | Explicit output file path |
| `--output-dir` | `-d` | — | Directory for `{input_stem}.fixed.{ext}` |
| `--in-place` | — | — | Overwrite the input file |
| `--overwrite` | — | — | Replace existing output (default: error if present) |

`--output`, `--output-dir`, and `--in-place` are mutually exclusive (exit 2 if more than one is set).

**Default output** (all `vd-fix-*`): `{input_dir}/{input_stem}.fixed.{ext}`

```text
meeting.txt  → meeting.fixed.txt
meeting.json → meeting.fixed.json
meeting.srt  → meeting.fixed.srt
```

Existing outputs → exit 2 unless `--overwrite` (or `--in-place`).

#### Behavior

**Rewrites only wording needed to lock terms to a canonical form.**

Changes **words** when a loaded dictionary / rule maps a variant to a canonical term:

- product names
- libraries / frameworks
- APIs
- protocols
- file formats
- companies
- project names
- abbreviations
- English identifiers (when listed)

Does not:

- restyle presentation (`vd-fix-casing`)
- repair open-ended ASR mishearings (`vd-fix-asr`)
- invent a canonical form not present in a loaded source
- translate
- rewrite sentences for style
- change segment boundaries / timestamps / speakers / ids / metadata

#### Guarantees

`vd-fix-terms` never changes:

- segment boundaries
- timestamps
- speaker labels
- ids
- metadata
- artifact type

It **may** change words, but **only inside transcript text spans**, and **only** to forms supported by loaded dictionaries / rules.

**Never invents** a canonical name without a dictionary / rule entry.

#### Options

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--language` | `-l` | `ru` | Language mode: `ru` (shipping focus — Russian + English insertions); `en` reserved; `de`, `auto` reserved |
| `--terms` | — | nearest `.voxdecoder` if present | Project assets from `vd-assets`. **Repeatable**. Explicit paths replace the default. Do not pass raw Office/PDF |
| `--no-shipping-lexicon` | — | — | Disable the shipping lexicon; use only `--terms` / config (corporate-only glossaries) |
| `--dry-run` | — | — | Print resolved options and exit (no rewrite) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` or `json` |
| `--quiet` | `-q` | — | Disable progress on stderr |

Prefer prepared assets from [`vd-assets`](../../process/vd-assets/). Default: nearest **`.voxdecoder/`** (walk up from the input). Override with `--terms`, `$VD_PROJECT_DIR`, or `VD_PROJECT_DIR=` in `.voxdecoder/env` / `.env`. The assets directory is the unit (`terms.yml` + `md/`). `--terms` is the **authoritative** project source for locking terminology (**repeatable**; left → right, **last wins**). It is not the same as `vd-fix-asr --context` (recognition hints).

**Shipping lexicon** is on by default. Disable with `--no-shipping-lexicon` for corporate-only dictionaries. Precedence (highest first): `--terms` → user config (future) → shipping lexicon. See [README.md](README.md#sources-precedence) and [STRUCTURE.md](STRUCTURE.md#source-precedence).

#### Examples

```bash
vd-fix-terms run -i /path/meeting.txt
vd-fix-terms -i /path/meeting.txt
# → /path/meeting.fixed.txt

vd-fix-terms run -i meeting.txt -o ./out/meeting.txt
vd-fix-terms run -i meeting.srt -d ./cleaned/
vd-fix-terms run -i meeting.txt --in-place
vd-fix-terms run -i meeting.txt
# → uses ./.voxdecoder when present
vd-fix-terms run -i meeting.txt --terms /other/assets
vd-fix-terms run -i meeting.txt --terms /other/assets --terms ./extra.yaml
vd-fix-terms run -i meeting.txt --terms ./corp.yaml --no-shipping-lexicon
vd-fix-terms run -i meeting.txt --language ru --progress=json
vd-fix-terms run -i meeting.txt --dry-run
vd-fix-terms run -i meeting.txt --dry-run --json
vd-fix-terms run -i meeting.txt -q
```

##### `--dry-run`

Prints the resolved plan and exits 0 (no rewrite).

Text (default):

```text
Input: /path/meeting.txt
Artifact type: txt
Output: /path/meeting.fixed.txt
Language: ru
Terms: ./.voxdecoder
Shipping lexicon: yes
In-place: off
Overwrite: off
```

Machine-readable (`--dry-run --json`):

```json
{
  "input": "/path/meeting.txt",
  "artifact_type": "txt",
  "output": "/path/meeting.fixed.txt",
  "language": "ru",
  "terms": ["./.voxdecoder"],
  "shipping_lexicon": true,
  "in_place": false,
  "overwrite": false
}
```

#### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Processing error |
| 2 | Invalid arguments or invalid CLI usage |
| 3 | Input file missing / unreadable / unsupported artifact type; `--terms` path missing / unreadable |
| 4 | Backend / lexicon failed to initialize (e.g. corrupt pack later) |

Exit 2 includes: unknown option, incompatible flags (`-o` with `-d` / `--in-place`), output exists without `--overwrite`, unknown `--language`.

---

### Possible future: `install` / `remove` / `list` / `info`

Only if optional downloadable term packs become necessary. Same UX shape as `vd-gigaam` / `vd-fix-casing` if/when added. Not required when shipping lexicon + `--terms` is enough.

---

### `vd-fix-terms config`

```bash
vd-fix-terms config list
vd-fix-terms config get language
vd-fix-terms config set language ru
vd-fix-terms config set in_place off
vd-fix-terms config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|-------------|
| `language` | `ru` | Language mode (`ru` / …) |
| `in_place` | `off` | Default to rewriting the input path |
| `progress` | `text` | `text` / `json` |

Priority: CLI > config > default.

(Optional later: default `terms` path list in config.)

---

## Progress

| Value | Description |
|-------|-------------|
| `text` | Human-readable progress on stderr (default) |
| `json` | NDJSON events on stderr (for GUI / scripts) |

Omit progress with `-q` / `--quiet`.

Stdout stays free (except `--dry-run` / config text).

Shared scheme with other CLIs via [`vd-progress`](../../../crates/vd-progress/): `start` → `phase`* → `done` | `error`.

### `run --progress=json`

```json
{"event":"start","input":"…","output":"…","artifact_type":"txt","language":"ru"}
{"event":"phase","phase":"loading","percent":5}
{"event":"phase","phase":"processing","percent":40,"span":1,"span_total":3}
{"event":"phase","phase":"processing","percent":70,"span":2,"span_total":3}
{"event":"phase","phase":"writing","percent":90}
{"event":"done","output":"/path/meeting.fixed.txt","duration_sec":0.8,"char_count":12400}
{"event":"error","code":"backend_init_failed","message":"…"}
```

`phase=processing` + `span` / `span_total` tracks span progress.

---

## Term sources (product shape)

Exact on-disk schema is an implementation detail. Product expectations:

- structured glossaries (`yaml` / `json`): variant → canonical (see [README.md](README.md#glossary-shape-illustrative))
- markdown / README / docs may contribute when they clearly define terms (extractor is backend-private)
- **shipping lexicon** covers common tech terminology for `--language ru` without files (off with `--no-shipping-lexicon`)
- **`--terms`** (repeatable; left → right, **last wins**) is the project override / extension path

**Precedence** (highest first): `--terms` → user config (future) → shipping lexicon. See [STRUCTURE.md](STRUCTURE.md#source-precedence).

---

## Public contract note

Dictionary format, matcher, and any future inference backend are intentionally **outside** the public CLI contract.
