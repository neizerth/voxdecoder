# vd-fix-overlap CLI

Remove duplicated speech introduced by diarization overlap.

**Status: implemented.** `run` reads a real diarized JSON/JSONL artifact,
runs detection, and reports candidate pairs. With `--apply` (or any output
flag), it removes the `drop` side of each pair and writes a fixed artifact.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-fix-overlap run` | Detect (and optionally remove) duplicated speech across speakers |
| `vd-fix-overlap config` | Default detection thresholds |

Shorthand: `vd-fix-overlap -i FILE` ≡ `vd-fix-overlap run -i FILE`.

---

## `vd-fix-overlap run`

### Input

A JSON (or JSONL) file whose turns carry speaker + timing + text together,
matching `vd-pipeline`'s `MeetingTurn` shape:

```json
{
  "turns": [
    {"speaker": "A", "start_sec": 1.0, "end_sec": 3.0, "text": "Let's deploy tomorrow."},
    {"speaker": "B", "start_sec": 1.2, "end_sec": 3.2, "text": "let's deploy tomorrow"}
  ]
}
```

Recognized keys (case-insensitive, first match wins): speaker —
`speaker` / `speaker_id` / `speaker_label`; timing — `start_sec` /
`start_time` / `start` and `end_sec` / `end_time` / `end`; text — same
`TEXT_KEYS` every `vd-fix-*` CLI uses (`text`, `content`, `transcript`,
`utterance`, `caption`, `sentence`, `line`). A "turn" is any array element
(or bare JSONL line) that is an object with a recognized text key —
speaker/timing are optional (missing timing means the turn is always
treated as temporally close to everything else, which only matters if your
input omits timestamps entirely).

Only JSON/JSONL are supported. `Txt`/`Md` have no multi-turn structure;
`Srt`/`Vtt` carry timing but no structural speaker field, so cross-speaker
duplication can't be verified — both exit 3 with "no speaker turns found".

| Argument | Short | Required | Description |
|----------|-------|----------|-------------|
| `--input` | `-i` | ✅ | Path to a diarized JSON/JSONL transcript |
| `--output` | `-o` | — | Explicit output file path (implies `--apply`) |
| `--output-dir` | `-d` | — | Directory for `{input_stem}.fixed.{ext}` (implies `--apply`) |
| `--in-place` | — | — | Overwrite the input file (implies `--apply`) |
| `--overwrite` | — | — | Replace existing output (default: error if present) |
| `--apply` | — | — | Remove detected duplicates and write a fixed artifact |
| `--similarity-threshold` | — | — | Override the configured similarity threshold (`[0.0, 1.0]`) |
| `--max-gap-ms` | — | — | Override the configured max temporal gap (milliseconds) |
| `--json` | — | — | Machine-readable report on stdout instead of the text summary |
| `--quiet` | `-q` | — | Suppress summary/status lines |

`--output`, `--output-dir`, and `--in-place` are mutually exclusive (exit 2
if more than one is set). Without `--apply` (and no output flag), `run` is
report-only — nothing is written, matching ADR 0012's "if uncertain,
preserve both copies" posture: you see what *would* be removed before
committing to it.

**Default output** (same pattern as every `vd-fix-*`):
`{input_dir}/{input_stem}.fixed.{ext}`

### Output

Text report (default):

```text
1 candidate duplicate pair(s) found (2 turns checked):
  [exact] keep=0 (A) drop=1 (B) similarity=1.00
```

or, when nothing is flagged:

```text
No duplicate speech detected (2 turns checked).
```

`--json`:

```json
[
  {
    "keep": 0,
    "keep_speaker": "A",
    "drop": 1,
    "drop_speaker": "B",
    "kind": "exact",
    "similarity": 1.0,
    "action": "remove"
  }
]
```

`keep` / `drop` are indices into the detected-turns list (in document
order). `kind` is `"exact"` (identical after normalization) or `"near"`
(at/above the similarity threshold, not identical). `action` is what
`--apply` would do (or did) to `drop`:

- `"remove"` — delete the whole turn (it has no content beyond what `keep`
  already has).
- `"trim"` — `drop` case-insensitively contains `keep`'s text as a clean
  prefix or suffix plus a genuine unique remainder (ADR 0012 §2 "partial
  duplicates"); deleting the whole turn would lose that remainder, so the
  turn survives with its text rewritten to just the remainder. A
  `trimmed_text` field carries the replacement text in this case.

A fuzzy (edit-distance) near-match with no clean prefix/suffix containment
always gets `"remove"` — trimming only ever applies to the deterministic
containment case.

With `--apply`, after the report: every `remove` pair's `drop` turn is
deleted, every `trim` pair's `drop` turn is rewritten to `trimmed_text`
(all its other fields — speaker, timing — stay untouched), and the result
is written to the resolved output path. If no pairs were found, nothing is
written — the input is already clean, there is no `.fixed.` file to
compare against.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success (including "no duplicates found") |
| 1 | Processing error (e.g. malformed input JSON) |
| 2 | Invalid arguments (`--similarity-threshold` outside `[0.0, 1.0]`, missing `-i`, conflicting output flags) |
| 3 | Input file missing / unreadable, or no speaker turns found in the artifact |

---

## `vd-fix-overlap config`

```bash
vd-fix-overlap config list
vd-fix-overlap config get similarity_threshold
vd-fix-overlap config set similarity_threshold 0.9
vd-fix-overlap config set max_gap_ms 250
vd-fix-overlap config path
```

| Key | Default | Description |
|-----|---------|--------------|
| `similarity_threshold` | `0.85` | Normalized edit-distance similarity in `[0.0, 1.0]` at/above which two spans count as near-duplicate |
| `max_gap_ms` | `500` | Max gap (ms) between two non-overlapping time ranges that still counts as "close" |

Priority: CLI > config > default.

---

## Public contract note

Trimming only ever fires for **deterministic prefix/suffix containment** —
`drop`'s text case-insensitively starts or ends with `keep`'s text plus a
real remainder. A fuzzy near-match with no clean boundary (e.g. a mid-text
typo) is never trimmed, only fully removed — matching ADR 0012 §2's "if
uncertain, preserve both copies" for anything less than that.
