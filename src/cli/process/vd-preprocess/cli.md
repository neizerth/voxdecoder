# vd-preprocess CLI

Universal **media filter-chain** executor: input media + ordered filters + provider(s) → prepared media.  
Standalone CLI **and** `use: preprocess` for the shared Executor.

**Status: implemented.**

---

## Architecture

```text
CLI flags / Job step (use: preprocess)
              ↓
         vd-preprocess
              ↓
        Prepared Media   (registered output)
```

Same binary / library for both surfaces. **No filters → error.**

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-preprocess run` | Apply filter chain to one media input |
| `vd-preprocess filters` | *(planned)* List catalog (groups · operations · providers) |
| `vd-preprocess config` | Defaults (provider, progress, binary paths, …) |
| `vd-preprocess validate` | *(planned)* Check filter chain without invoking providers |

Shorthand (planned): `-i FILE` without subcommand inserts `run`.

---

## `run`

```bash
# fails — no filters
vd-preprocess run -i meeting.wav
# error: no filters specified

vd-preprocess run -i meeting.wav \
  --filter trim-silence \
  --filter normalize \
  --filter 'speed:factor=1.15'

vd-preprocess run -i meeting.mkv \
  --provider ffmpeg \
  --chain ./prepare.yaml \
  --dry-run --json

vd-preprocess run -i track.wav \
  --chain ./denoise-only.yaml \
  -o track.prepared.wav
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--input` / `-i` | `-i` | — | Media path (**required**) |
| `--chain` | `-c` | — | YAML/JSON file with `filters:` (+ optional `provider:`) |
| `--filter` | `-f` | — | Repeatable short filter (`name` or `name:key=val,…`) |
| `--provider` | — | config / `ffmpeg` | Default provider for short `--filter` / `type:` |
| `--output` / `-o` | `-o` | next to input / stem rule | Prepared media path |
| `--output-dir` / `-d` | `-d` | — | Directory for default output name |
| `--dry-run` | — | — | Plan only (no DSP invoke) |
| `--json` | — | — | With `--dry-run`: plan JSON |
| `--progress` | — | `text` | `text` \| `json` |
| `--quiet` / `-q` | `-q` | — | No progress |
| `--overwrite` | — | — | Replace existing output |

At least one of `--chain` or `--filter` is required (non-empty chain after merge).

### Chain file

```yaml
# prepare.yaml
provider: ffmpeg
filters:
  - type: extract-audio
  - type: resample
    rate: 16000
  - type: mono
  - type: trim-silence
    min_duration: 500ms
  - type: normalize
```

Explicit providers:

```yaml
filters:
  - provider: ffmpeg
    operation: normalize
  - provider: deepfilternet
    operation: enhance
  - provider: ffmpeg
    operation: speed
    factor: 1.1
```

### Job step

```yaml
- use: preprocess
  id: prepared
  input: meeting.wav
  options:
    provider: ffmpeg
    filters:
      - type: extract-audio
      - type: resample
        rate: 16000
      - type: mono
      - type: normalize
      - type: denoise
      - type: speed
        factor: 1.1

- use: transcribe
  input: prepared
```

Meeting — normalize only on room (keep timeline):

```yaml
- use: preprocess
  id: room_audio
  input: room.wav
  options:
    filters:
      - type: normalize
      - type: denoise

- use: diarize
  input: room_audio
```

Participant — may speed for cheaper ASR (timeline not shared with diarize):

```yaml
- use: preprocess
  id: p1_audio
  input: participant1.wav
  options:
    filters:
      - type: normalize
      - type: speed
        factor: 1.15

- use: transcribe
  input: p1_audio
```

MCP sends the same Job fragment — never an ffmpeg SDK.

---

## Filter catalog

Groups for GUI / `filters` listing:

| Group | Operations |
|-------|------------|
| **Media** | `extract-audio`, `convert`, `resample`, `mono`, `stereo` |
| **Audio** | `normalize`, `denoise`, `highpass`, `lowpass`, `compressor` |
| **Timing** | `speed`, `trim-silence`, `trim`, `chunk` |
| **Channels** | `split-channels`, `merge-channels` |

### Common params

| Operation | Params (examples) |
|-----------|-------------------|
| `resample` | `rate` (Hz) |
| `speed` | `factor` |
| `trim-silence` | `min_duration`, `threshold` |
| `trim` | `start`, `end` / `duration` |
| `highpass` / `lowpass` | `cutoff_hz` |
| `denoise` / `enhance` | provider-specific (`model`, `strength`, …) |

Exact param schema is provider-owned; plan-time validation rejects unknown keys when the provider declares a schema.

---

## Providers

| `provider` | Role | Status (target) |
|------------|------|-----------------|
| `ffmpeg` | Default local media toolbox | primary |
| `sox` | Alternate DSP | optional |
| `deepfilternet` | ML enhance / denoise | optional assets |
| `rnnoise` | Noise suppression | optional |
| `demucs` | Source separation | optional |
| `stub` | Deterministic CI / dry pipelines | tests |

```bash
vd-preprocess config set provider ffmpeg
vd-preprocess config get provider
```

Binary / model paths: env (`FFMPEG`, `VD_PREPROCESS_*`) + config — same pattern as other tools.

---

## Config

```bash
vd-preprocess config list
vd-preprocess config get provider
vd-preprocess config set provider ffmpeg
vd-preprocess config path
```

First-class keys (planned):

| Key | Role |
|-----|------|
| `provider` | Default for `type:` / `--filter` sugar |
| `progress` | `text` \| `json` |
| provider-specific | e.g. ffmpeg path, deepfilternet model dir |

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `2` | Usage — missing input, **empty filters**, bad flags |
| `3` | I/O / missing file |
| `4` | Provider / filter execution failure |
| `5` | Unsupported operation for provider |

---

## Dry-run

```bash
vd-preprocess run -i meeting.wav --chain prepare.yaml --dry-run --json
```

Emits `ExecutionPlan`: default provider, expanded filters (provider + operation + params), resolved binaries/models, output path — **no** media writes.

---

## Progress

Stderr via [`vd-progress`](../../../crates/vd-progress/): `start` → per-filter `phase` (optional) → `done` / `error`. `--quiet` disables. `--progress=json` for machines.

---

## Relationship to default `vd-pipeline`

```bash
vd-pipeline run -i meeting.ogg
# builder inserts use: preprocess (default ASR chain) before transcribe
```

Explicit Job may omit preprocess or place it on selected branches only. See [README § Pipeline placement](README.md#pipeline-placement).
