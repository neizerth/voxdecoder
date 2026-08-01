# vd-giga CLI

Rust CLI for local transcription via [GigaAM](https://github.com/salute-developers/GigaAM) (Candle in-process). Model-specific notes: [README.md](README.md).

First-class load and inference flags (`-m`, `--device`, `--flash`, `--no-fp16-encoder`, …). Background jobs belong to `vd-srv`; progress goes to stderr via `--progress`.

---

## Commands

| Command | Description |
|---------|-------------|
| `vd-giga run` | Transcribe a local audio/video file |
| `vd-giga config` | Default settings |
| `vd-giga install` | Download a GigaAM model |
| `vd-giga remove` | Remove an installed model |
| `vd-giga list` | List models |
| `vd-giga info` | Show model metadata |

Shorthand: `vd-giga -i FILE` ≡ `vd-giga run -i FILE`.

---

## 1. `vd-giga run`

```bash
# Minimum: result next to the audio file
vd-giga run -i /path/meeting.ogg
vd-giga -i /path/meeting.ogg
# → /path/meeting.txt

# Model and device
vd-giga run -i voice.mp3 -m v3_e2e_rnnt --device cuda

# Full GigaAM load options
vd-giga run -i call.wav -m v2_rnnt \
  --device cuda --flash \
  --download-root ~/models/gigaam

# Disable FP16 encoder (on by default)
vd-giga run -i call.wav -m v2_rnnt --no-fp16-encoder

# Word timestamps → written into --format json / --segments
vd-giga run -i podcast.mp3 -m v3_ctc --word-timestamps --format json
vd-giga run -i podcast.mp3 -m v3_ctc --word-timestamps --segments

# Output
vd-giga run -i meeting.ogg -o ./out/result.txt
# → ./out/result.txt
# → ./out/result.segments.json  (with --segments)
vd-giga run -i meeting.ogg -d ./transcripts/
# → ./transcripts/meeting.txt
vd-giga run -i lecture.mp4 --format srt -d ./subs/
vd-giga run -i meeting.ogg --overwrite

# Preview resolved options (no transcription)
vd-giga run -i voice.mp3 -m v2_rnnt --flash --dry-run
vd-giga run -i voice.mp3 -m v2_rnnt --flash --dry-run --json

# Progress for GUI / scripts
vd-giga run -i voice.mp3 --progress=json
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
| `--progress` | — | — | Progress on stderr: `text` or `json` (off if omitted) |

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
| `--download-root` | — | managed cache | Checkpoint directory |

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
vd-giga run -i voice.wav -m ~/models/gigaam/v3_e2e_rnnt.ckpt
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
Download root: …/vd-giga/models
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
  "download_root": "…/vd-giga/models",
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

## 2. `vd-giga install` / `remove` / `list` / `info`

```bash
vd-giga install v2_rnnt
vd-giga install v3_e2e_rnnt --download-root ~/models/gigaam
vd-giga install --all
vd-giga install v2_rnnt --progress=json

vd-giga remove v2_rnnt
vd-giga remove v2_rnnt -y
vd-giga remove v2_rnnt --yes

vd-giga list
vd-giga list --all
vd-giga list --json

vd-giga info v3_e2e_rnnt
vd-giga info v3_e2e_rnnt --json
```

### `install`

| Argument | Short | Description |
|----------|-------|-------------|
| `MODEL` | — | Catalog name (`v2_rnnt`, …); omit with `--all` |
| `--all` | — | Install every catalog model |
| `--download-root` | — | Checkpoint directory (same flag as `run`) |
| `--progress` | — | `text` or `json` (off if omitted) |

Default checkpoints: `…/vd-giga/models/<name>.ckpt`  
(override: `VD_GIGA_MODELS_DIR` or `--download-root`).

### `remove`

| Argument | Short | Description |
|----------|-------|-------------|
| `MODEL` | — | Catalog name or path |
| `--yes` | `-y` | Assume yes; do not prompt for confirmation |

### `list`

```text
Installed

✓ v2_rnnt
✓ v3_ctc
✓ v3_e2e_rnnt
```

`list --all`:

```text
✓ v2_rnnt
✓ v3_ctc
○ v3_rnnt
○ v3_e2e_ctc
…
```

| Argument | Description |
|----------|-------------|
| `--all` | Include catalog models that are not installed (`○`) |
| `--json` | Machine-readable list |

### `info`

Shows catalog / install metadata without loading the model into GPU:

```text
name:       v3_e2e_rnnt
decoder:    rnnt
line:       v3 e2e
language:   ru
installed:  yes
downloaded: yes
path:       …/vd-giga/models/v3_e2e_rnnt.ckpt
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
vd-giga run -i file.ogg -m v2_rnnt
```

---

## 3. `vd-giga config`

```bash
vd-giga config list
vd-giga config get model
vd-giga config set model v3_e2e_rnnt
vd-giga config set device cuda
vd-giga config set flash on
vd-giga config set fp16_encoder off
vd-giga config path
```

Booleans use `on` / `off`.

| Key | Default | Description |
|-----|---------|-------------|
| `model` | `v2_rnnt` | GigaAM name or path to `.ckpt` / `.pt` |
| `device` | `auto` | cpu / cuda / auto |
| `fp16_encoder` | `on` | FP16 encoder (CLI: `--no-fp16-encoder`) |
| `flash` | `off` | FlashAttention (CLI: `--flash`) |
| `download_root` | — | Checkpoint directory (empty → managed) |
| `word_timestamps` | `off` | Word-level timestamps |
| `format` | `txt` | txt / json / srt / vtt |

Priority: CLI > config > default.

---

## 4. Progress (`--progress`)

Same flag for `run` and `install`:

| Value | Description |
|-------|-------------|
| *(omit)* | No progress (default) |
| `text` | Human-readable progress on stderr |
| `json` | NDJSON events on stderr (for GUI / scripts) |

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
{"event":"start","model":"v2_rnnt","path":"…/vd-giga/models"}
{"event":"phase","phase":"downloading","percent":42,"bytes_done":123456789,"bytes_total":300000000}
{"event":"done","model":"v2_rnnt","path":"…/vd-giga/models/v2_rnnt.ckpt"}
{"event":"error","code":"download_failed","message":"…"}
```
