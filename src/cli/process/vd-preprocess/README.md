# vd-preprocess — filter-chain executor for media

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI surface: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [vd-pipeline](../vd-pipeline/) · [vd-meeting](../vd-meeting/) · [vd-postprocess](../vd-postprocess/).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/), [`vd-output`](../../../crates/vd-output/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented.** Workspace member: `src/cli/process/vd-preprocess`. Default provider for CI / dry Jobs: `stub` (copy-through). `ffmpeg` wired for real DSP when available (`VD_FFMPEG` or PATH). Heavy backends (`deepfilternet`, `rnnoise`, `demucs`, …) typed but not wired yet.

## Core rule

```text
vd-preprocess is a universal media filter-chain executor.

Media + Filters + Provider(s) → Prepared Media.

It is the DSP/media counterpart of vd-postprocess (recipe chain).
Without filters, it does nothing (and errors).
```

> **vd-preprocess prepares audio/video for downstream capabilities.**  
> It does not transcribe, diarize, fix text, or invent product modes like “meeting enhance”. Those are **filter chains** the user (or default Job builder) owns. One binary; many chains.  
> **Provider** means *DSP / media backend* — ffmpeg, sox, DeepFilterNet, RNNoise, Demucs, … — not “one fixed CLI flag set”.

## Contract

```text
Media (audio / video)
      +
Filter chain
      +
Provider(s)
      ↓
Prepared Media
```

| Surface | Role |
|---------|------|
| **CLI** (`vd-preprocess run`) | Human UX: input + filter chain + provider defaults |
| **`use: preprocess`** | Same implementation, scheduled by [`vd-pipeline`](../vd-pipeline/) Executor |
| **MCP / `vd-srv`** | Submit a Job step; never own ffmpeg / ML SDKs |

`vd-preprocess` knows nothing about meetings, ASR engines, or speaker identity. Planners only add Job step(s) with `options.filters` (+ optional default `provider`).

---

## Twin of `vd-postprocess`

| | [`vd-postprocess`](../vd-postprocess/) | **`vd-preprocess`** |
|--|----------------------------------------|----------------------|
| Abstraction | Recipe chain | **Filter chain** |
| Domain | Text / structured artifacts | Media (audio / video) |
| Unit of work | Recipe document | Filter step (`provider` + `operation`) |
| Empty chain | Error | Error |
| Placement | Anywhere in the DAG | Anywhere in the DAG |

```text
vd-preprocess   =  graph of DSP / media filters   (provider + operation)
vd-pipeline     =  DAG of capabilities
vd-postprocess  =  graph of recipe nodes          (ExecutionRunner per node; recipe-portable)
```

Platform flow:

```text
Media → Filter Graph (preprocess) → Artifacts → Capability DAG (pipeline) → Artifacts → Recipe Graph (postprocess) → Derived
```

Do **not** grow a flat flag surface (`--speed`, `--normalize`, `--denoise`, …) as the product model. Flags may sugar common filters; the **Job contract is the chain**.

---

## Why a filter chain, not flags

Flags encode a fixed product. A chain stays a **capability**:

```yaml
# Job step
- use: preprocess
  id: prepared
  input: meeting.wav
  options:
    filters:
      - provider: ffmpeg
        operation: extract-audio

      - provider: ffmpeg
        operation: resample
        rate: 16000

      - provider: ffmpeg
        operation: mono

      - provider: ffmpeg
        operation: normalize

      - provider: deepfilternet
        operation: enhance

      - provider: ffmpeg
        operation: speed
        factor: 1.1
```

Short form (default provider from step / config, usually `ffmpeg`):

```yaml
filters:
  - type: trim-silence
    min_duration: 500ms
  - type: normalize
  - type: speed
    factor: 1.15
```

`type: X` ≡ `provider: <default>`, `operation: X`.

Meeting branch without timeline distortion:

```yaml
filters:
  - type: normalize
  - type: denoise
```

No trim / speed — clocks stay aligned for diarize / merge.

---

## Not only the first step — a normal capability

`preprocess` is **not** glued exclusively to the head of a linear pipeline. It is a regular DAG node. Default single-file Jobs put it **first** (e.g. trim silence → normalize → ASR). Meeting Jobs may attach a **different** chain **per branch**:

```text
participant1.wav
        │
   preprocess          # e.g. normalize + speed
        │
   transcribe
        │
     fix-*
        │
        ┐

participant2.wav
        │
   preprocess
        │
   transcribe
        │
     fix-*
        │
        ┘

merged.wav / room
        │
   preprocess          # e.g. normalize only — no speed
        │
    diarize
```

That is why [`vd-pipeline`](../vd-pipeline/) is a **DAG** Executor — each branch owns its own media preparation. Timeline-sensitive branches must not rewrite time (no `speed` / aggressive `trim-silence` unless the planner accepts skew).

---

## Filter groups (GUI-ready)

Filters are **operations**. Groups are for UX / docs / discovery — not separate executors.

```text
Media
    extract-audio
    convert
    resample
    mono
    stereo

Audio
    normalize
    denoise
    highpass
    lowpass
    compressor

Timing
    speed
    trim-silence
    trim
    chunk

Channels
    split-channels
    merge-channels
```

A GUI can group the catalog by these buckets. New providers may add operations under the same groups without changing Job schema.

---

## Providers are extensible

Not a closed enum forever:

```text
ffmpeg          (default local toolbox)
sox
deepfilternet
rnnoise
demucs
…
```

Each filter names **who** runs it and **what** operation:

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

Step-level default (like `transcribe.engine`):

```yaml
- use: preprocess
  options:
    provider: ffmpeg          # default for short `type:` filters
    filters:
      - type: normalize
      - type: resample
        rate: 16000
```

Analogy:

```yaml
transcribe:
  engine: gigaam

preprocess:
  provider: ffmpeg
```

Auth / model paths / binary discovery via env + config — never baked into Meeting Model. MCP picks `filters` / `provider` in Job JSON; it does not shell out itself.

---

## Capability: `preprocess`

| `use` | Responsibility |
|-------|----------------|
| **`preprocess`** | Media → prepared media via **user (or builder) filter chain** |
| `transcribe` | Get text from audio/video |
| `prepare-context` | Get project knowledge |
| `fix-*` | Improve text |
| `diarize` | Speaker timeline |
| `meeting-merge` | Combine meeting artifacts |
| `postprocess` | Produce **new** artifacts from existing ones via **user recipes** |

Default linear cleanup Job (builder inserts preprocess first):

```text
preprocess → transcribe → prepare-context → fix-* → (postprocess…)
```

Typical default filter intent for ASR:

```yaml
filters:
  - type: extract-audio      # if video
  - type: resample
    rate: 16000
  - type: mono
  - type: trim-silence       # optional; default on for single-file ASR Jobs
  - type: normalize
```

Exact default chain is owned by the Job builder (`vd-pipeline` CLI defaults / `vd-meeting` planner) — **not** hard-coded inside the preprocess binary as a silent product mode.

---

## Pipeline placement

```text
# default single-source Job
preprocess → transcribe → …

# meeting DAG (per branch)
track → preprocess → transcribe → fix-*
room  → preprocess → diarize → meeting-merge
```

`vd-meeting` may emit different chains per role (`room` vs participant). Timeline branches should avoid `speed` / silence removal that desyncs speakers unless `purposes` allow it.

---

## Boundaries

| Tool | Owns |
|------|------|
| [`vd-pipeline`](../vd-pipeline/) | Job DAG + Executor; binds `preprocess`; **default Job** inserts preprocess early |
| **`vd-preprocess`** | Load filter chain · resolve providers · run DSP/media steps · write / register prepared media |
| Job builders | Which filters (defaults vs meeting policy) |
| MCP | Job JSON only |

`vd-preprocess` never:

- ships a hidden “always denoise” product mode when no filters given
- invents filters when the chain is empty
- assumes every provider is ffmpeg
- owns Meeting Model, diarization, or ASR
- replaces `postprocess` / `fix-*`

---

## Guarantees

1. **No filters → error** (exit 2), not a silent default chain inside the binary.
2. **CLI ≡ capability** — same options in flags and Job `options`.
3. **Filter = provider + operation** (+ operation-specific fields); `type:` is sugar for the step/default provider.
4. **Chain order is significant** — executed strictly left-to-right (unless a filter documents otherwise).
5. **Inputs / outputs are artifacts** — prepared media registers like any other step (`id` → path).
6. **Dry-run emits ExecutionPlan** after provider resolve — no DSP invoke.
7. **Local-first** — default providers run on-machine; optional asset download for ML denoisers, never upload of user media by default.

---

## Status note

Implemented. CI default provider is `stub` (copy-through). Use `--provider ffmpeg` (or config) for real DSP when `ffmpeg` is on `PATH` / `$VD_FFMPEG`. Optional ML providers remain typed but unwired.