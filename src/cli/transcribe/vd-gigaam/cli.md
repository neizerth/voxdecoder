# vd-gigaam CLI

Rust CLI for local transcription via [GigaAM](https://github.com/salute-developers/GigaAM) (Candle in-process). Model-specific notes: [README.md](README.md).

First-class load and inference flags (`-m`, `--device`, `--flash`, `--no-fp16-encoder`, …). Background jobs belong to `vd-srv`; progress goes to stderr via `--progress`.

---

## Commands

| Command | Description |
|---------|-------------|
| `vd-gigaam run` | Transcribe a local audio/video file |
| `vd-gigaam config` | Default settings |
| `vd-gigaam install` | Download a GigaAM model |
| `vd-gigaam remove` | Remove an installed model |
| `vd-gigaam list` | List models |
| `vd-gigaam info` | Show model metadata |

Shorthand: `vd-gigaam -i FILE` ≡ `vd-gigaam run -i FILE`.

---

## 1. `vd-gigaam run`

```bash
# Minimum: result next to the audio file
vd-gigaam run -i /path/meeting.ogg
vd-gigaam -i /path/meeting.ogg
# → /path/meeting.txt

# Model and device
vd-gigaam run -i voice.mp3 -m v3_e2e_rnnt --device cuda

# Full GigaAM load options
vd-gigaam run -i call.wav -m v2_rnnt \
  --device cuda --flash \
  --download-root ~/models/gigaam

# Disable FP16 encoder (on by default)
vd-gigaam run -i call.wav -m v2_rnnt --no-fp16-encoder

# Word timestamps → written into --format json / --segments
vd-gigaam run -i podcast.mp3 -m v3_ctc --word-timestamps --format json
vd-gigaam run -i podcast.mp3 -m v3_ctc --word-timestamps --segments

# Output
vd-gigaam run -i meeting.ogg -o ./out/result.txt
# → ./out/result.txt
# → ./out/result.segments.json  (with --segments)
vd-gigaam run -i meeting.ogg -d ./transcripts/
# → ./transcripts/meeting.txt
vd-gigaam run -i lecture.mp4 --format srt -d ./subs/
vd-gigaam run -i meeting.ogg --overwrite

# Preview resolved options (no transcription)
vd-gigaam run -i voice.mp3 -m v2_rnnt --flash --dry-run
vd-gigaam run -i voice.mp3 -m v2_rnnt --flash --dry-run --json

# Progress for GUI / scripts
vd-gigaam run -i voice.mp3 --progress=json
```

### Input / output

| Argument | Short | Required | Description |
|----------|-------|----------|-------------|
| `--input` | `-i` | ✅ | Path to audio or video |
| `--output` | `-o` | — | Explicit output file path |
| `--output-dir` | `-d` | — | Directory for `{input_stem}.{ext}` |
| `--format` | — | — | `txt`, `json`, `srt`, `vtt` (default: `txt`) |
| `--segments` | — | — | Also write `{output_stem}.segments.json` next to the main output |
| `--overwrite` | — | — | Replace existing output files (default: error if present) |
| `--dry-run` | — | — | Print resolved options and exit (no transcription) |
| `--json` | — | — | With `--dry-run`: machine-readable plan on stdout |
| `--progress` | — | — | Progress on stderr: `text` or `json` (default: `text`) |
| `--quiet` | `-q` | — | Disable progress on stderr |

`--output` and `--output-dir` are mutually exclusive (exit 2 if both are set).

**Default output:** `{input_dir}/{input_stem}.{ext}` where `{ext}` follows `--format` (`.txt`, `.json`, `.srt`, `.vtt`).

| `--format` | Contents |
|------------|----------|
| `txt` | Plain transcript text |
| `json` | Structured result: text, segments, optional words |
| `srt` | Subtitles |
| `vtt` | WebVTT subtitles |

`--segments` is derived from the **resolved main output path**, not from the input:

| Main output | Sidecar |
|-------------|---------|
| `-o out/foo.txt` | `out/foo.segments.json` |
| `-d out/` + input `meeting.ogg` | `out/meeting.segments.json` |
| default + input `/path/meeting.ogg` | `/path/meeting.segments.json` |

Same schema as `--format json` body; independent of `--format`.

Existing outputs → exit 2 unless `--overwrite`.

### GigaAM model (Rust / Candle load)

Flags mirror the reference load API; the binary loads weights in-process (SafeTensors / converted checkpoint), not via Python:

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--model` | `-m` | `v2_rnnt` | Catalog name or path to `.ckpt` / `.pt` (converted / loaded in Rust) |
| `--device` | — | `auto` | `cpu`, `cuda`, `auto` (Metal where available) |
| `--no-fp16-encoder` | — | — | Disable FP16 encoder (default: on) |
| `--flash` | — | — | Enable FlashAttention (default: off) |
| `--download-root` | — | GigaAM cache | Checkpoint directory |

ASR catalog:

| Name | Notes |
|------|-------|
| `v3_e2e_rnnt`, `v3_e2e_ctc` | v3 end-to-end |
| `v3_rnnt`, `v3_ctc` | v3 |
| `v2_rnnt`, `v2_ctc` | v2 (default: `v2_rnnt`) |
| `v1_rnnt`, `v1_ctc` | v1 |

Short aliases: `rnnt` → `v2_rnnt`, `ctc` → `v2_ctc`, `e2e_rnnt` → `v3_e2e_rnnt`, `e2e_ctc` → `v3_e2e_ctc`.

Local checkpoint:

```bash
vd-gigaam run -i voice.wav -m ~/models/gigaam/v3_e2e_rnnt.ckpt
```

### Inference

| Argument | Default | Description |
|----------|---------|-------------|
| `--word-timestamps` | off | Request word-level timestamps |

`--word-timestamps` only has effect when words can land somewhere:

- `--format json` — words included in the JSON output
- `--segments` — words included in the sidecar

With `--format txt|srt|vtt` and without `--segments`, the flag is rejected (exit 2): words would be computed and discarded.

### `--dry-run`

Prints the resolved plan and exits 0 (no transcription).

Text (default):

```
Model: v2_rnnt
Device: cuda
Flash: on
FP16 encoder: on
Download root: ~/.cache/gigaam
Output: /path/meeting.txt
Segments: /path/meeting.segments.json
Overwrite: off
Word timestamps: on
```

Machine-readable (`--dry-run --json`):

```json
{
  "model": "v2_rnnt",
  "device": "cuda",
  "flash": true,
  "fp16_encoder": true,
  "download_root": "~/.cache/gigaam",
  "output": "/path/meeting.txt",
  "segments": "/path/meeting.segments.json",
  "overwrite": false,
  "word_timestamps": true
}
```

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Transcription error |
| 2 | Invalid arguments or invalid CLI usage |
| 3 | Input file missing / unreadable |
| 4 | Model loading failed (missing deps, bad checkpoint, CUDA, …) |

Exit 2 includes: unknown option, incompatible flags (`-o` with `-d`, `--word-timestamps` without a sink), output exists without `--overwrite`.

---

## 2. `vd-gigaam install` / `remove` / `list` / `info`

```bash
vd-gigaam install v2_rnnt
vd-gigaam install v3_e2e_rnnt --download-root ~/models/gigaam
vd-gigaam install --all
vd-gigaam install v2_rnnt --progress=json
vd-gigaam install v2_rnnt -q

vd-gigaam remove v2_rnnt
vd-gigaam remove v2_rnnt -y
vd-gigaam remove v2_rnnt --yes

vd-gigaam list
vd-gigaam list --all
vd-gigaam list --format json

vd-gigaam info v3_e2e_rnnt
vd-gigaam info v3_e2e_rnnt --json
```

### `install`

| Argument | Short | Description |
|----------|-------|-------------|
| `MODEL` | — | Catalog name (`v2_rnnt`, …); omit with `--all` |
| `--all` | — | Install every catalog model |
| `--download-root` | — | Checkpoint directory (same flag as `run`) |
| `--force` | — | Re-download / reconvert even if already installed |
| `--progress` | — | `text` or `json` (default: `text`) |
| `--quiet` | `-q` | Disable progress on stderr |

Default models dir (= Python GigaAM cache):

| Platform | Path |
|----------|------|
| Linux / macOS | `~/.cache/gigaam` |
| Linux (XDG) | `$XDG_CACHE_HOME/gigaam` |
| Windows | `%LOCALAPPDATA%\gigaam` |

Override: `VD_GIGAAM_MODELS_DIR`, `config set download_root`, or `--download-root`.

Interrupted `*.tmp` files are deleted on the next install. Already-converted SafeTensors → no-op (`already installed`).

When `--download-root` points elsewhere, a catalog `.ckpt` already in the GigaAM cache is reused (no second CDN download); converted weights still land in the chosen download root.

### `remove`

| Argument | Short | Description |
|----------|-------|-------------|
| `MODEL` | — | Catalog name or path |
| `--yes` | `-y` | Assume yes; do not prompt for confirmation |

### `list`

```text
Models dir: ~/.cache/gigaam

Available

✓ v2_rnnt          ready
· v3_e2e_ctc       ckpt
· v3_e2e_rnnt      ckpt
```

`list --all`:

```text
Models dir: ~/.cache/gigaam

✓ v2_rnnt          ready
· v3_e2e_ctc       ckpt
○ v3_ctc           missing
…
```

When `download_root` differs from the GigaAM cache, list also prints `GigaAM cache: …` and may label some rows `ckpt (GigaAM cache)`.

Marks: `✓` converted (run-ready), `·` `.ckpt` present, `○` missing.

| Argument | Description |
|----------|-------------|
| `--all` | Include catalog models that are not installed (`○`) |
| `--format` | Output format: `text` (default) or `json` |

### `info`

Shows catalog / install metadata without loading the model into GPU:

```text
name:       v3_e2e_rnnt
decoder:    rnnt
line:       v3 e2e
language:   ru
installed:  yes
downloaded: yes
path:       …/.cache/gigaam/v3_e2e_rnnt.ckpt
size:       874 MiB
sha256:     a1b2c3…
```

| Argument | Description |
|----------|-------------|
| `MODEL` | Catalog name or local checkpoint path |
| `--json` | Machine-readable metadata |

`size` / `sha256` are reported when known (local file or catalog checksum).

After install:

```bash
vd-gigaam run -i file.ogg -m v2_rnnt
```

---

## 3. `vd-gigaam config`

```bash
vd-gigaam config list
vd-gigaam config get model
vd-gigaam config set model v3_e2e_rnnt
vd-gigaam config set device cuda
vd-gigaam config set flash on
vd-gigaam config set fp16_encoder off
vd-gigaam config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|-------------|
| `model` | `v2_rnnt` | GigaAM name or path to `.ckpt` / `.pt` |
| `device` | `auto` | cpu / cuda / auto |
| `fp16_encoder` | `on` | FP16 encoder (CLI: `--no-fp16-encoder`) |
| `flash` | `off` | FlashAttention (CLI: `--flash`) |
| `download_root` | — | Checkpoint directory (empty → GigaAM cache) |
| `word_timestamps` | `off` | Word-level timestamps |
| `format` | `txt` | txt / json / srt / vtt |

Priority: CLI > config > default.

---

## 4. Progress (`--progress`)

Same flag for `run` and `install`:

| Value | Description |
|-------|-------------|
| `text` | Human-readable progress on stderr (default) |
| `json` | NDJSON events on stderr (for GUI / scripts) |

Omit progress with `-q` / `--quiet`.

Stdout stays free (except `--dry-run` / `info` / `list` text). Example for `run --progress=json`:

```json
{"event":"start","input":"…","output":"…","model":"v2_rnnt","device":"cuda"}
{"event":"phase","phase":"loading_model","percent":5}
{"event":"phase","phase":"transcribing","percent":55,"segment":2,"segment_total":4}
{"event":"done","output":"/path/meeting.txt","duration_sec":89.2,"char_count":12400}
{"event":"error","code":"model_load_failed","message":"…"}
```

Example for `install --progress=json`:

```json
{"event":"start","model":"v2_rnnt","path":"…/.cache/gigaam"}
{"event":"phase","phase":"downloading","percent":42,"bytes_done":123456789,"bytes_total":300000000}
{"event":"done","model":"v2_rnnt","path":"…/.cache/gigaam/v2_rnnt"}
{"event":"error","code":"download_failed","message":"…"}
```
