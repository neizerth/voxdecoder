# vd-fix-casing CLI

Local presentation fixer (punctuation, casing, whitespace).

Rewrites only presentation. The input artifact type and structure are preserved.

**Packs are optional** for the built-in rules backend: `run` works without `install`. Optional packs (`install ru` / `en`) cache or override the embedded lexicon (same UX shape as `vd-gigaam install`).

Product notes: [README.md](README.md). Shared UX with other `vd-fix-*` CLIs. Background jobs: `vd-srv`.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-fix-casing run` | Fix presentation in a local text artifact |
| `vd-fix-casing install` | Download / install a language pack |
| `vd-fix-casing remove` | Remove an installed pack |
| `vd-fix-casing list` | List packs |
| `vd-fix-casing info` | Show pack metadata |
| `vd-fix-casing config` | Default settings |

Shorthand: `vd-fix-casing -i FILE` ≡ `vd-fix-casing run -i FILE`.

---

## Commands

### `vd-fix-casing run`

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

Changes **presentation only**:

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

#### Guarantees

`vd-fix-casing` never changes:

- words
- segment boundaries
- timestamps
- speaker labels
- ids
- metadata

Only presentation of transcript text is rewritten.

#### Options

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--language` | `-l` | `ru` | Language pack: `ru`, `en` (shipping); `de`, `auto` reserved |
| `--download-root` | — | platform cache | Models directory (same as `install`) |
| `--dry-run` | — | — | Print resolved options and exit (no rewrite) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` or `json` |
| `--quiet` | `-q` | — | Disable progress on stderr |

Without an installed pack, `run` uses the **embedded** lexicon for `--language`. If a pack is installed, it is preferred.

#### Examples

```bash
vd-fix-casing run -i /path/meeting.txt
vd-fix-casing -i /path/meeting.txt
# → /path/meeting.fixed.txt

vd-fix-casing install ru   # optional: cache / override embedded lexicon
vd-fix-casing install en

vd-fix-casing run -i meeting.txt -o ./out/meeting.txt
vd-fix-casing run -i meeting.srt -d ./cleaned/
vd-fix-casing run -i meeting.txt --in-place
vd-fix-casing run -i meeting.txt --language en --progress=json
vd-fix-casing run -i meeting.txt --dry-run
vd-fix-casing run -i meeting.txt --dry-run --json
vd-fix-casing run -i meeting.txt -q
```

##### `--dry-run`

Prints the resolved plan and exits 0 (no rewrite). Reports whether an optional pack is installed (`no (builtin)` vs `yes`).

Text (default):

```text
Input: /path/meeting.txt
Artifact type: txt
Output: /path/meeting.fixed.txt
Language: ru
Model: ru
Pack installed: no (builtin)
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
  "model": "ru",
  "installed": false,
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
| 3 | Input file missing / unreadable / unsupported artifact type |
| 4 | Inference backend failed to initialize (e.g. corrupt installed pack) |

Exit 2 includes: unknown option, incompatible flags (`-o` with `-d` / `--in-place`), output exists without `--overwrite`.

Missing pack is **not** exit 4 — builtin lexicon is used.

---

### `vd-fix-casing install` / `remove` / `list` / `info`

```bash
vd-fix-casing install ru
vd-fix-casing install en --download-root ~/models/vd-fix-casing
vd-fix-casing install --all
vd-fix-casing install ru --progress=json
vd-fix-casing install ru -q
vd-fix-casing install ru --force

vd-fix-casing remove ru
vd-fix-casing remove ru -y

vd-fix-casing list
vd-fix-casing list --all
vd-fix-casing list --format json

vd-fix-casing info ru
vd-fix-casing info ru --json
```

#### `install`

Optional for the rules backend. Caches / overrides the embedded lexicon for a language (same UX as `vd-gigaam install`).

| Argument | Short | Description |
|----------|-------|-------------|
| `MODEL` | — | Catalog name (`ru`, `en`); omit with `--all` |
| `--all` | — | Install every shipping catalog pack |
| `--download-root` | — | Models directory |
| `--force` | — | Reinstall even if already present |
| `--progress` | — | `text` or `json` (default: `text`) |
| `--quiet` | `-q` | Disable progress on stderr |

Default models dir (platform cache):

| Platform | Path |
|----------|------|
| Linux | `~/.cache/vd-fix-casing/models` (or `$XDG_CACHE_HOME/…`) |
| macOS | `~/Library/Caches/vd-fix-casing/models` |
| Windows | `%LOCALAPPDATA%\vd-fix-casing\cache\models` |

Override: `VD_FIX_CASING_MODELS_DIR`, `config set download_root`, or `--download-root`.

Interrupted `*.tmp` files are deleted on the next install. Already installed → no-op (`already installed`) unless `--force`.

#### `remove`

| Argument | Short | Description |
|----------|-------|-------------|
| `MODEL` | — | Catalog name |
| `--yes` | `-y` | Assume yes; do not prompt |

#### `list`

```text
Models dir: …/Caches/vd-fix-casing/models

✓ ru               ready
✓ en               ready
○ de               missing (not shipping)
```

| Argument | Description |
|----------|-------------|
| `--all` | Include catalog entries that are not installed / not shipping |
| `--format` | Output format: `text` (default) or `json` |

#### `info`

```text
name:       ru
language:   ru
backend:    rules
version:    1
installed:  yes
path:       …/vd-fix-casing/models/ru
size:       4 KiB
```

| Argument | Description |
|----------|-------------|
| `MODEL` | Catalog name |
| `--json` | Machine-readable metadata |

---

### `vd-fix-casing config`

```bash
vd-fix-casing config list
vd-fix-casing config get language
vd-fix-casing config set language ru
vd-fix-casing config set download_root ~/models/vd-fix-casing
vd-fix-casing config set in_place off
vd-fix-casing config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|-------------|
| `language` | `ru` | Language pack (`ru` / `en` / …) |
| `download_root` | — | Models directory (empty → platform cache) |
| `in_place` | `off` | Default to rewriting the input path |
| `progress` | `text` | `text` / `json` |

Priority: CLI > config > default.

---

## Progress

Same flag for `run` and `install`:

| Value | Description |
|-------|-------------|
| `text` | Human-readable progress on stderr (default) |
| `json` | NDJSON events on stderr (for GUI / scripts) |

Omit progress with `-q` / `--quiet`.

Stdout stays free (except `--dry-run` / `info` / `list` text).

### `run --progress=json`

```json
{"event":"start","input":"…","output":"…","artifact_type":"txt","language":"ru","model":"ru"}
{"event":"phase","phase":"loading","percent":5}
{"event":"phase","phase":"processing","percent":40,"span":1,"span_total":3}
{"event":"phase","phase":"processing","percent":70,"span":2,"span_total":3}
{"event":"phase","phase":"writing","percent":90}
{"event":"done","output":"/path/meeting.fixed.txt","duration_sec":1.2,"char_count":12400}
{"event":"error","code":"backend_init_failed","message":"…"}
```

`phase=processing` + `span` / `span_total` tracks span progress (0–100). Meaningful for multi-span artifacts (`srt` / `json` / …); single-span `txt` still emits percent.

### `install --progress=json`

```json
{"event":"start","model":"ru","path":"…/vd-fix-casing/models"}
{"event":"phase","phase":"downloading","percent":42,"bytes_done":12345,"bytes_total":30000}
{"event":"done","model":"ru","path":"…/vd-fix-casing/models/ru"}
{"event":"error","code":"download_failed","message":"…"}
```
