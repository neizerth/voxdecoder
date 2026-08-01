# vd-diarize CLI

Local-first speaker diarization: standalone CLI **and** `use: diarize` for the shared Executor.

**Status: implemented.**

Product: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md). Process: [../README.md](../README.md).

---

## Architecture

```text
CLI flags / Job step (use: diarize)
              ↓
         vd-diarize
              ↓
      SpeakerTimeline
```

Same binary / library for both surfaces.

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-diarize run` | Diarize one audio file → SpeakerTimeline |
| `vd-diarize install` | Install backend **assets** (models, configs, …) |
| `vd-diarize remove` | Remove installed assets for a provider |
| `vd-diarize list` | Installed providers / assets |
| `vd-diarize info` | Details for one provider |
| `vd-diarize config` | Defaults |

Shorthand: `vd-diarize -i FILE` ≡ `vd-diarize run -i FILE`.

---

## `run`

```bash
vd-diarize run -i meeting.wav
vd-diarize run -i meeting.wav -o meeting.diarization.json
vd-diarize run -i meeting.wav --backend pyannote --model speaker-diarization-3.1
vd-diarize run -i meeting.wav --dry-run --json
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--input` | `-i` | — | Audio file (required) |
| `--output` | `-o` | next to input | SpeakerTimeline export path |
| `--output-dir` | `-d` | — | Directory for default artifact name |
| `--backend` | — | config | Provider: `pyannote` \| `nemo` \| … |
| `--model` | `-m` | provider default | Model id within the provider |
| `--device` | — | auto | Local device hint |
| `--dry-run` | — | — | Plan only |
| `--json` | — | — | With `--dry-run`: plan JSON |
| `--progress` | — | `text` | `text` \| `json` |
| `--quiet` | `-q` | — | No progress |
| `--overwrite` | — | — | Replace existing artifact |

Job step:

```yaml
- use: diarize
  id: timeline
  input: meeting.wav
  options:
    backend:
      provider: pyannote
      model: speaker-diarization-3.1
    device: cuda
```

---

## Assets (`install` / `remove` / `list` / `info`)

Install **assets** for a backend — not a vague “download models” only. Assets may include segmentation / embedding / clustering weights, configs, rules, tokenizers.

```bash
vd-diarize install pyannote
vd-diarize install nemo
vd-diarize list
vd-diarize info pyannote
vd-diarize remove pyannote
```

| Source | Role |
|--------|------|
| Hugging Face | Optional download channel |
| Local directory | Explicit path / asset root |
| Enterprise mirror | Configurable base URL |
| Bundled assets | When shipped |

After install, **run needs no network**. Audio never uploads.

---

## Artifact output

Canonical type: **SpeakerTimeline**. See [README.md](README.md#what-is-a-diarization-artifact).

Speaker ids (`S0`, …) are **local to this artifact**. Resolving to real names is `meeting-merge`.

---

## Behavior

1. Resolve input; fail if missing (exit 3).
2. Resolve backend (`provider` + `model`: CLI > Job options > config).
3. Resolve assets (fail with install hint if missing).
4. Load backend locally.
5. `--dry-run` → plan → exit 0.
6. Infer → write SpeakerTimeline (with required `backend` metadata).
7. Exit 0 / 1.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success or dry-run |
| 1 | Inference / I/O failure |
| 2 | Bad options / unknown provider / usage |
| 3 | Missing input / missing installed assets |

---

## Config

```bash
vd-diarize config list
vd-diarize config get backend.provider
vd-diarize config set backend.provider pyannote
vd-diarize config set backend.model speaker-diarization-3.1
vd-diarize config path
```

| Key | Default | Description |
|-----|---------|-------------|
| `backend.provider` | `stub` | Default provider (`stub` works offline; `pyannote` / `nemo` after install + runtime) |
| `backend.model` | provider default | Default model id |
| `progress` | `text` | Progress mode |
| `download_root` | platform cache | Asset cache |

`$VD_DIARIZE_CONFIG` or platform config dir.

Priority: CLI > Job `options` > config > default.

---

## Public contract note

**Local inference · SpeakerTimeline · swappable backends (`provider` + `model`).**  
Cloud diarization APIs are out of scope. Identity mapping is out of scope.
