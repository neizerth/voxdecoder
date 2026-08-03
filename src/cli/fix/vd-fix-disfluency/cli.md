# vd-fix-disfluency CLI

Local speech-disfluency cleanup (fillers, repeated filler runs, empty hesitations, false starts).

**Removes speech noise. Never removes information.** The input artifact type and structure are preserved.

**Status: implemented (deterministic rules).** No model, no download.

**Language default: `ru`** — mirrors `vd-fix-asr`'s ru-priority default.

Product notes: [README.md](README.md). Shared UX with other `vd-fix-*` CLIs. Background jobs: `vd-srv`.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-fix-disfluency run` | Remove speech disfluencies from a local text artifact |
| `vd-fix-disfluency config` | Default settings |

Shorthand: `vd-fix-disfluency -i FILE` ≡ `vd-fix-disfluency run -i FILE`.

---

## Commands

### `vd-fix-disfluency run`

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

**Removes speech noise. Never removes information.**

- filler syllables (`эээ`, `ммм`, `эм` / `um`, `uh`, `erm`) — all modes except `off`
- repeated filler runs — collapsed to one instance (`light`) or removed entirely (`normal`/`aggressive`)
- empty hesitations (`Ну... эээ... да...` → `Ну, да...`)
- false starts (`Я... я думаю...` → `Я думаю...`) — `normal`/`aggressive` only

Never touches a hardcoded protected-phrase list of meaningful discourse markers (`ну да`, `ну конечно`, `вот именно`, English equivalents), and never changes segment boundaries / timestamps / speakers / ids / metadata.

#### Options

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--language` | `-l` | `ru` | `ru` (ru-priority default), `en`, `de` (reserved → ru tables), `auto` (reserved → ru tables) |
| `--mode` | `-m` | `light` | `off` \| `light` \| `normal` \| `aggressive` |
| `--no-fillers` | — | — | Force `mode=off` for this run regardless of `--mode` / config (maps to config's `remove_fillers=off`) |
| `--dry-run` | — | — | Print resolved options and exit (no rewrite) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` or `json` |
| `--quiet` | `-q` | — | Disable progress on stderr |

#### Examples

```bash
vd-fix-disfluency run -i /path/meeting.txt
vd-fix-disfluency -i /path/meeting.txt
# → /path/meeting.fixed.txt

vd-fix-disfluency run -i meeting.txt -o ./out/meeting.txt
vd-fix-disfluency run -i meeting.srt -d ./cleaned/
vd-fix-disfluency run -i meeting.txt --in-place
vd-fix-disfluency run -i meeting.txt --mode normal
vd-fix-disfluency run -i meeting.txt --mode aggressive --language en
vd-fix-disfluency run -i meeting.txt --no-fillers
vd-fix-disfluency run -i meeting.txt --dry-run
vd-fix-disfluency run -i meeting.txt --dry-run --json
vd-fix-disfluency run -i meeting.txt -q
```

##### `--dry-run`

Prints the resolved plan and exits 0 (no rewrite).

Text (default):

```text
Input: /path/meeting.txt
Artifact type: txt
Output: /path/meeting.fixed.txt
Language: ru
Mode: light
Remove fillers: on
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
  "mode": "light",
  "remove_fillers": true,
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
| 4 | Backend failed to initialize (reserved — the current rules backend cannot fail) |

Exit 2 includes: unknown option, incompatible flags (`-o` with `-d` / `--in-place`), output exists without `--overwrite`, unknown `--language` / `--mode`.

---

### `vd-fix-disfluency config`

```bash
vd-fix-disfluency config list
vd-fix-disfluency config get mode
vd-fix-disfluency config set mode normal
vd-fix-disfluency config set remove_fillers off
vd-fix-disfluency config set language ru
vd-fix-disfluency config set in_place off
vd-fix-disfluency config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|--------------|
| `language` | `ru` | Language mode (`ru` / `en` / …) |
| `mode` | `light` | `off` \| `light` \| `normal` \| `aggressive` |
| `remove_fillers` | `on` | Master switch; `off` forces effective mode to `off` regardless of `mode` |
| `in_place` | `off` | Default to rewriting the input path |
| `progress` | `text` | `text` / `json` |

Priority: CLI > config > default.

---

## Progress

| Value | Description |
|-------|-------------|
| `text` | Human-readable progress on stderr (default) |
| `json` | NDJSON events on stderr (for GUI / scripts) |

Omit progress with `-q` / `--quiet`. Stdout stays free (except `--dry-run` / config text).

### `run --progress=json`

```json
{"event":"start","input":"…","output":"…","artifact_type":"txt","language":"ru"}
{"event":"phase","phase":"loading","percent":5}
{"event":"phase","phase":"processing","percent":40,"span":1,"span_total":3}
{"event":"phase","phase":"writing","percent":90}
{"event":"done","output":"/path/meeting.fixed.txt","duration_sec":0.2,"char_count":540}
{"event":"error","code":"load_failed","message":"…"}
```

---

## Public contract note

Rule tables (filler lists, protected phrases) are an implementation detail and may grow; the CLI contract is language, mode, and the disfluency-removal guarantee — not the exact token lists.
