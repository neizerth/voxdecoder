# vd-fix-layout CLI

Local layout fixer for readable long-form text (**ru** / **en** / **auto** in v1).

```text
Never changes lexical content.
```

Only whitespace and paragraph / block boundaries may change. The input artifact type is preserved.

**Packs are optional** for builtin language baselines: `run` works without `install`. Optional packs (`install ru` / `en`) deepen cue lists / local models (same UX as `vd-gigaam` / `vd-fix-casing`).

Product notes: [README.md](README.md). Shared UX with other `vd-fix-*` CLIs. Background jobs: `vd-srv`.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-fix-layout run` | Apply layout (paragraph breaks in v1) to a local text artifact |
| `vd-fix-layout install` | Download / install a language pack (`ru`, `en`) |
| `vd-fix-layout remove` | Remove an installed pack |
| `vd-fix-layout list` | List packs |
| `vd-fix-layout info` | Show pack metadata |
| `vd-fix-layout config` | Default settings |

Shorthand: `vd-fix-layout -i FILE` ≡ `vd-fix-layout run -i FILE`.

---

## Commands

### `vd-fix-layout run`

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

Also accepts `vd-*` native artifacts; type is preserved. Useful on transcripts **and** on long-form outputs (e.g. `summary.md` after `vd-postprocess`).

| Argument | Short | Required | Description |
|----------|-------|----------|-------------|
| `--input` | `-i` | ✅ | Path to a text artifact |
| `--output` | `-o` | — | Explicit output file path |
| `--output-dir` | `-d` | — | Directory for `{input_stem}.fixed.{ext}` |
| `--in-place` | — | — | Overwrite the input file |
| `--overwrite` | — | — | Replace existing output (default: error if present) |

`--output`, `--output-dir`, and `--in-place` are mutually exclusive (exit 2 if more than one is set).

**Default output** (all `vd-fix-*`): `{input_dir}/{input_stem}.fixed.{ext}`

Existing outputs → exit 2 unless `--overwrite` (or `--in-place`).

#### Behavior

Changes **layout only** (v1 = paragraphs):

- inserts paragraph breaks between sentence groups
- language-specialized for `--language ru` \| `en` \| `auto`
- optional **TimeMap** structural hints when a TimeMap is bound

Does not:

- change lexical content
- repair ASR errors
- normalize terminology
- translate
- rewrite sentences
- invent headings or list markup

#### Guarantees

```text
Never changes lexical content.
```

Also never changes speaker labels, ids, or metadata.

```text
Paragraph boundaries never split
a timed segment or speaker label.
```

Only whitespace and paragraph / block boundaries may change.

#### Options

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--language` | `-l` | `auto` | `ru` \| `en` \| `auto`. Other codes → exit 2 |
| `--density` | — | `normal` | `compact` \| `normal` \| `relaxed` |
| `--timemap` | — | — | Optional local path when running standalone; product contract is abstract binding (see TimeMap) |
| `--no-timemap` | — | — | Ignore TimeMap even if available |
| `--download-root` | — | platform cache | Models / packs directory |
| `--dry-run` | — | — | Print resolved options and exit (no rewrite) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` or `json` |
| `--quiet` | `-q` | — | Disable progress on stderr |

There is **no** `--segments` flag.

```text
TimeMap provides optional structural hints
(pauses, timing, speaker transitions).

Layout remains fully functional
without a TimeMap.
```

#### Language `auto`

Resolution order:

1. Language on the **artifact** (if declared)
2. Language from the bound **TimeMap** (if any)
3. **Autodetection** over text → `ru` or `en`
4. **Config** fallback (`ru` if still unresolved)

Dry-run reports both requested (`auto`) and **resolved** (`ru` / `en`).

#### Examples

```bash
vd-fix-layout run -i /path/meeting.fixed.txt
vd-fix-layout -i /path/meeting.fixed.txt --language auto

vd-fix-layout install ru
vd-fix-layout install en

vd-fix-layout run -i summary.md --language en --density relaxed
vd-fix-layout run -i talk.txt --language auto --no-timemap
vd-fix-layout run -i talk.txt --dry-run --json
```

##### `--dry-run`

```text
Input: /path/meeting.txt
Artifact type: txt
Output: /path/meeting.fixed.txt
Language: auto
Language resolved: ru
Model: ru
Pack installed: no (builtin)
Density: normal
TimeMap:
  source: artifact
In-place: off
Overwrite: off
```

```json
{
  "input": "/path/meeting.txt",
  "artifact_type": "txt",
  "output": "/path/meeting.fixed.txt",
  "language": "auto",
  "language_resolved": "ru",
  "model": "ru",
  "installed": false,
  "density": "normal",
  "timemap": {
    "source": "artifact"
  },
  "in_place": false,
  "overwrite": false
}
```

When no TimeMap is bound:

```json
"timemap": null
```

or `"timemap": { "source": "none" }` — pick one in implementation; prefer `null` for absent.

Do **not** require dry-run to print a filesystem path. Standalone `--timemap PATH` may still appear as `"source": "cli"` for operators.

#### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Processing error |
| 2 | Invalid arguments / usage (incl. unsupported `--language` / `--density`) |
| 3 | Input missing / unreadable / unsupported type |
| 4 | Inference backend failed to initialize (e.g. corrupt pack) |

Missing pack is **not** exit 4. Missing TimeMap is **not** an error — layout proceeds without structural hints.

---

### `vd-fix-layout install` / `remove` / `list` / `info`

```bash
vd-fix-layout install ru
vd-fix-layout install en --download-root ~/models/vd-fix-layout
vd-fix-layout install --all
vd-fix-layout remove ru
vd-fix-layout list
vd-fix-layout info ru --json
```

| Code | Status | Notes |
|------|--------|-------|
| `ru` | shipping | Russian-specialized local tools |
| `en` | shipping | English-specialized local tools |

`auto` is not a pack — it resolves to `ru` or `en` at run time.

Default models dir: platform cache under `vd-fix-layout/models`. Override: `VD_FIX_LAYOUT_MODELS_DIR`, `--download-root`, `config set download_root`.

---

### `vd-fix-layout config`

```bash
vd-fix-layout config list
vd-fix-layout config get language
vd-fix-layout config set language auto
vd-fix-layout config set paragraph_density relaxed
vd-fix-layout config set use_timemap true
vd-fix-layout config path
```

Priority: **CLI > config file > default**.

| Key | Default | Description |
|-----|---------|-------------|
| `language` | `auto` | `ru` \| `en` \| `auto` |
| `download_root` | platform cache | Packs directory |
| `paragraph_density` | `normal` | `compact` \| `normal` \| `relaxed` |
| `use_timemap` | `true` | Bind TimeMap structural hints when available |
| `progress` | `text` | `text` \| `json` |

Low-level sentence counts stay **inside** the pack/backend — not public config keys.

---

## Progress

Stderr via `vd-progress`: `start` → `phase`* → `done` | `error`.

Phases:

```text
loading
analyzing
layout
writing
```

`analyzing` = choosing break candidates (cues / TimeMap / density). `layout` = applying whitespace / boundaries.

---

## Job integration (planned)

Capability: `fix-layout` (kebab-case in Job YAML).

Prefer ArtifactRef inputs so the Executor can bind a TimeMap from the Job graph without a path flag:

```yaml
- use: fix-layout
  inputs:
    transcript: transcript
  options:
    language: auto
    density: normal
```

The Executor may attach TimeMap from the same Job (preprocess / ASR / diarize artifact) when `use_timemap` is on.

Default audio Job may append after `fix-terms` and before postprocess recipes once the crate ships.

Also usable standalone on postprocess outputs (any long-form text artifact).
