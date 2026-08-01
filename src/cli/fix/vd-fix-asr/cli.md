# vd-fix-asr CLI

Local ASR wording fixer (misheard words, homophones, ru/en mix-ups).

**Rewrites only wording needed to restore meaning.** The input artifact type and structure are preserved.

**Status: implemented (builtin rules backend).** Pack downloads are still a possible future — see below.

**Language default: `ru`** — Russian with English insertions (see [TODO-languages.md](TODO-languages.md)).

Downloads / language packs are **not** part of the committed public surface yet. See *Possible future commands* below.

Product notes: [README.md](README.md). Shared UX with other `vd-fix-*` CLIs. Background jobs: `vd-srv`.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-fix-asr run` | Repair ASR wording in a local text artifact |
| `vd-fix-asr config` | Default settings |

Shorthand: `vd-fix-asr -i FILE` ≡ `vd-fix-asr run -i FILE`.

### Possible future commands

`install` / `remove` / `list` / `info` **may** appear later if optional downloadable assets are needed — same shape as `vd-gigaam` / `vd-fix-casing`, and only if processing cannot stay fully builtin. Do not rely on them in product docs yet.

---

## Commands

### `vd-fix-asr run`

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

**Rewrites only wording needed to restore meaning.**

Changes **words / local meaning** when recognition was wrong:

- misrecognized words
- homophones
- Russian / English mix-ups
- technical terms distorted by ASR
- obvious errors that break meaning

Does not:

- restyle presentation (`vd-fix-casing`)
- force canonical terminology (`vd-fix-terms`)
- translate
- rewrite sentences for style
- change segment boundaries / timestamps / speakers / ids / metadata
- invent information unsupported by the transcript, neighbors, or `--context`

#### Guarantees

`vd-fix-asr` never changes:

- segment boundaries
- timestamps
- speaker labels
- ids
- metadata
- artifact type

It **may** change words, but **only inside transcript text spans**.

**Never invents information** that is not supported by the transcript, neighboring context, or supplied `--context` materials.

#### Options

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--language` | `-l` | `ru` | Language mode: `ru` (shipping focus — Russian + English insertions); `en` reserved; `de`, `auto` reserved |
| `--context` | — | — | Additional project material (file or directory). Repeatable. May be docs, glossaries, dictionaries, source code, wiki, RFCs, … |
| `--context-neighbors` | — | `1` | How many neighboring segments to consider (0 = span only) |
| `--dry-run` | — | — | Print resolved options and exit (no rewrite) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | `text` | Progress on stderr: `text` or `json` |
| `--quiet` | `-q` | — | Disable progress on stderr |

`--context` is intentionally broad: additional project documentation, glossaries, dictionaries, code trees, and similar materials used as recognition hints — **not** canonical term locking (`vd-fix-terms`).

#### Examples

```bash
vd-fix-asr run -i /path/meeting.txt
vd-fix-asr -i /path/meeting.txt
# → /path/meeting.fixed.txt

vd-fix-asr run -i meeting.txt -o ./out/meeting.txt
vd-fix-asr run -i meeting.srt -d ./cleaned/
vd-fix-asr run -i meeting.txt --in-place
vd-fix-asr run -i meeting.txt --context ./docs --context ./glossary.yaml
vd-fix-asr run -i meeting.txt --context ./README.md --context-neighbors 2
vd-fix-asr run -i meeting.txt --language ru --progress=json
vd-fix-asr run -i meeting.txt --dry-run
vd-fix-asr run -i meeting.txt --dry-run --json
vd-fix-asr run -i meeting.txt -q
```

##### `--dry-run`

Prints the resolved plan and exits 0 (no rewrite).

Text (default):

```text
Input: /path/meeting.txt
Artifact type: txt
Output: /path/meeting.fixed.txt
Language: ru
Context: ./docs, ./glossary.yaml
Context neighbors: 1
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
  "context": ["./docs", "./glossary.yaml"],
  "context_neighbors": 1,
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

Exit 2 includes: unknown option, incompatible flags (`-o` with `-d` / `--in-place`), output exists without `--overwrite`, unknown `--language`.

---

### Possible future: `install` / `remove` / `list` / `info`

Only if optional downloadable assets become necessary. Same UX shape as `vd-gigaam` / `vd-fix-casing` if/when added. Not required for the wording contract above.

---

### `vd-fix-asr config`

```bash
vd-fix-asr config list
vd-fix-asr config get language
vd-fix-asr config set language ru
vd-fix-asr config set context_neighbors 2
vd-fix-asr config set in_place off
vd-fix-asr config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|-------------|
| `language` | `ru` | Language mode (`ru` / …) |
| `context_neighbors` | `1` | Neighboring segments for context |
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

Stdout stays free (except `--dry-run` / config text).

### `run --progress=json`

```json
{"event":"start","input":"…","output":"…","artifact_type":"txt","language":"ru"}
{"event":"loading","percent":5}
{"event":"processing","percent":40,"span":1,"span_total":3}
{"event":"processing","percent":70,"span":2,"span_total":3}
{"event":"writing","percent":90}
{"event":"done","output":"/path/meeting.fixed.txt","duration_sec":2.4,"char_count":12400}
{"event":"error","code":"backend_init_failed","message":"…"}
```

`processing.percent` tracks span progress (0–100). Meaningful for multi-span artifacts (`srt` / `json` / …); single-span `txt` still emits percent.

---

## Public contract note

Model family, inference runtime, and backend implementation are intentionally **outside** the public CLI contract.
