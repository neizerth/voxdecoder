# vd-diarize — who spoke when

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI surface: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [vd-meeting](../vd-meeting/) · [vd-pipeline](../vd-pipeline/).  
Shared crates (planned): [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented.** Workspace member: `src/cli/process/vd-diarize`. Default backend: `stub` (deterministic; CI / dry pipelines). `pyannote` / `nemo`: assets installable; local runtime TBD.

## Core rule

```text
vd-diarize is a standalone CLI and the implementation of the
`diarize` capability used by the shared Executor.

It answers one question: who spoke when (anonymously).

It does not transcribe, name people, fix text, or build meetings.
It is never part of a transcript / fix-* branch.
```

> **vd-diarize is a local-first speaker diarization capability.**  
> It identifies when anonymous speakers talk. Inference always runs locally. Model assets may be installed from Hugging Face or another source and are cached locally. No audio is transmitted over the network.

## Contract

```text
audio
   ↓
vd-diarize   (CLI  ≡  capability: diarize)
   ↓
SpeakerTimeline   (canonical artifact)
```

| Surface | Role |
|---------|------|
| **CLI** (`vd-diarize run`) | Human UX for one file |
| **`use: diarize`** | Same implementation, scheduled by [`vd-pipeline`](../vd-pipeline/) Executor |
| **[`vd-meeting`](../vd-meeting/) Planner** | May add a `diarize` branch to a Job; never reimplements diarization |

```text
Meeting → Job → diarize → SpeakerTimeline → meeting-merge
```

`vd-diarize` knows nothing about Meeting. `vd-meeting` knows nothing about pyannote internals.

The bound **backend** (`provider` + `model`) is an implementation detail — like `transcribe` + `options.engine`. The public promise is **SpeakerTimeline**.

---

## Local-first

```text
Inference is always local.

Model assets may be downloaded once,
cached locally,
and reused.

No audio is transmitted.
```

| Rule | Detail |
|------|--------|
| Inference | Always on the local machine |
| Network | Only when installing **optional** assets |
| Cache | Assets stored locally and reused across CLI / Executor runs |
| Upload | Never — no audio / embeddings sent to a service |
| Cloud APIs | Out of scope |

Runtime may use PyTorch, ONNX Runtime, Candle (future), or another **local** inference stack. That choice is not part of the Job contract.

---

## What is a Diarization Artifact

Canonical type: **SpeakerTimeline** (`ArtifactType::SpeakerTimeline`).  
File exports (`meeting.diarization.json`, …) are serializations of that type.

```text
SpeakerTimeline describes speech activity only.

It contains:
  - anonymous speaker ids (local to this artifact)
  - time ranges (segments)
  - optional confidence
  - optional overlaps
  - optional structured embeddings
  - optional speech regions
  - required backend metadata (provider, model, …)

It never contains transcript text.
```

```yaml
version: 1

audio:
  path: meeting.wav

speakers:
  - id: S0
  - id: S1

segments:
  - speaker: S0
    start: 0.18
    end: 2.74
    confidence: 0.98
  - speaker: S1
    start: 2.81
    end: 6.12
    confidence: 0.95

overlaps: []
speech_regions: []

embeddings:                    # optional, structured — not a binary blob
  per_speaker:
    - speaker: S0
      model: ecapa
      vector: [/* … */]

backend:                       # required
  provider: pyannote
  model: speaker-diarization-3.1
  version: "3.1"
  device: cuda
```

---

## Speaker identity

```text
Speaker ids are local to one SpeakerTimeline.

S0 in file A is unrelated to S0 in file B.

Resolving S0 → Alice is the responsibility of the
meeting-merge capability — never of vd-diarize.
```

---

## Responsibilities

- Accept **one** audio recording
- Build a speaker / speech-activity timeline
- Emit **SpeakerTimeline**

## Guarantees

`vd-diarize` never:

- recognizes speech (ASR)
- invents real names (`S0` / `S1`, not “Alice”)
- treats speaker ids as global across files
- merges multiple tracks
- runs `fix-*` or transcript branches
- exports a meeting transcript
- uploads audio to the cloud

---

## Supported backends

The CLI does **not** promise a single engine. Backends are swappable via:

```yaml
backend:
  provider: pyannote
  model: speaker-diarization-3.1
```

| Provider | Role |
|----------|------|
| **pyannote** | General-purpose diarization (segmentation, overlap, clustering) — primary orientation |
| **nemo** | Alternative backend (e.g. MSDD / Sortformer) |
| **speechbrain** / **wespeaker** | Often used as embedding components inside a provider stack |

WhisperX is **not** a diarization backend (Whisper + pyannote). Resemblyzer is not a production target.

### Implementation options (typical stack)

Exact combination is an implementation detail.

| Component | Candidates |
|-----------|------------|
| Segmentation / overlap | pyannote.audio |
| Speaker embeddings | WeSpeaker, SpeechBrain ECAPA |
| Clustering | pyannote.audio, NVIDIA NeMo |
| End-to-end (optional) | NVIDIA NeMo Sortformer |

### Local model assets

Assets are **backend-specific**. `install` means **install assets** (models, configs, clustering rules, tokenizers, …) — not a single opaque “download models” blob.

```text
Assets may include:
  • segmentation models
  • speaker embedding models
  • clustering models / rules
  • configuration

Assets are cached locally and shared between CLI runs.
```

Distribution channels (not runtimes): Hugging Face, local directory, enterprise mirror, bundled packs.

```bash
vd-diarize install pyannote
vd-diarize install nemo
vd-diarize list
vd-diarize info pyannote
vd-diarize remove pyannote
```

After install, inference needs **no** network.

---

## In a Job DAG

```text
Merged.wav → diarize → SpeakerTimeline ──────────────┐
Alice.wav  → transcript branch → alice.transcript ──┼→ meeting-merge capability
Bob.wav    → transcript branch → bob.transcript ────┘
```

```yaml
- use: diarize
  id: timeline
  input: meeting.wav
  options:
    backend:
      provider: pyannote
      model: speaker-diarization-3.1
```

---

## Boundaries

| Tool | Owns |
|------|------|
| **`vd-diarize`** | One audio → SpeakerTimeline (`S*` local) |
| [`vd-pipeline`](../vd-pipeline/) | Shared Executor (`use: diarize`) |
| [`vd-meeting`](../vd-meeting/) | May plan a `diarize` branch; `meeting-merge` maps `S*` → names |

Full CLI / packs / exit codes: [cli.md](cli.md).
