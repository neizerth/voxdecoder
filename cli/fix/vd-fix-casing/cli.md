# vd-fix-casing CLI

Local presentation fixer (punctuation, casing, whitespace).

Rewrites only presentation. The input artifact type and structure are preserved.

Product notes: [README.md](README.md). Shared UX with other `vd-fix-*` CLIs. Background jobs: `vd-srv`.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-fix-casing run` | Fix presentation in a local text artifact |
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
| `--language` | `-l` | `ru` | Language hint: `ru`, `en`, `de`, `auto`, … |
| `--dry-run` | — | — | Print resolved options and exit (no rewrite) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` or `json` |
| `--quiet` | `-q` | — | Disable progress on stderr |

#### Examples

```bash
# Minimum: result next to the input
vd-fix-casing run -i /path/meeting.txt
vd-fix-casing -i /path/meeting.txt
# → /path/meeting.fixed.txt

# Explicit output
vd-fix-casing run -i meeting.txt -o ./out/meeting.txt
vd-fix-casing run -i meeting.srt -d ./cleaned/
# → ./cleaned/meeting.fixed.srt

# In place
vd-fix-casing run -i meeting.txt --in-place

# Structured artifacts (presentation of text fields only)
vd-fix-casing run -i meeting.json
vd-fix-casing run -i meeting.segments.json
vd-fix-casing run -i subs.vtt --overwrite

# Language hint (reserved; default ru)
vd-fix-casing run -i meeting.txt --language ru

# Preview resolved options (no rewrite)
vd-fix-casing run -i meeting.txt --dry-run
vd-fix-casing run -i meeting.txt --dry-run --json

# Progress for GUI / scripts
vd-fix-casing run -i meeting.txt --progress=json
vd-fix-casing run -i meeting.txt -q
```

##### `--dry-run`

Prints the resolved plan and exits 0 (no rewrite).

Text (default):

```text
Input: /path/meeting.txt
Artifact type: txt
Output: /path/meeting.fixed.txt
Language: ru
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
| 4 | Inference backend failed to initialize |

Exit 2 includes: unknown option, incompatible flags (`-o` with `-d` / `--in-place`), output exists without `--overwrite`.

---

### `vd-fix-casing config`

```bash
vd-fix-casing config list
vd-fix-casing config get language
vd-fix-casing config set language ru
vd-fix-casing config set in_place off
vd-fix-casing config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|-------------|
| `language` | `ru` | Language hint (`ru` / `en` / `de` / `auto`, …) |
| `in_place` | `off` | Default to rewriting the input path |
| `progress` | `text` | `text` / `json` |

Priority: CLI > config > default.

---

## Progress

| Value | Description |
|-------|-------------|
| `text` | Human-readable progress on stderr (default) |
| `json` | NDJSON events on stderr (for GUI / scripts) |

Omit progress with `-q` / `--quiet`.

Stdout stays free (except `--dry-run` text/JSON). Example for `run --progress=json`:

```json
{"event":"start","input":"…","output":"…","artifact_type":"txt","language":"ru"}
{"event":"loading"}
{"event":"processing","percent":40}
{"event":"writing","percent":90}
{"event":"done","output":"/path/meeting.fixed.txt","duration_sec":1.2,"char_count":12400}
{"event":"error","code":"unsupported_type","message":"…"}
```
