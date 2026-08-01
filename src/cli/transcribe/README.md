# Transcription CLIs

Local speech-to-text as **one binary per model family**, not a shared multi-model engine.

**Runtime basis:** Rust. Each CLI is a Rust binary that loads and runs its model through Candle (and model-specific code). **No Python in the product runtime** — users install a binary + weights only.

Python may exist only as **maintainer/CI tooling** (one-shot `.ckpt` → SafeTensors/GGUF conversion, golden tensor dumps). It is never a dependency of `vd-gigaam` / `vd-whisper` and is not required to build or run released binaries if converted weights are published.

---

## Design choice

| Approach | Our take |
|----------|----------|
| One engine + shared model trait / flat `--model-type` | Rejected |
| Separate CLIs, options mirror each model API | **This** (`vd-gigaam`, `vd-whisper`, …) |

Why separate CLIs:

- Each model has its own mel / decoder / load knobs. A shared flag surface either lies or explodes.
- Defaults and validation stay honest (`--flash` only where it exists; HTK mel only for GigaAM, etc.).
- Shipping / deps stay narrow: install GigaAM stack without pulling Whisper (and vice versa).
- `vd-srv` can call whichever binary the job requests; the queue does not need a plugin registry.

Shared UX conventions across binaries (I/O, `--progress`, config priority) are fine. Shared *inference* abstractions are not the goal.

---

## Binaries

| Binary | Model family | Spec |
|--------|--------------|------|
| `vd-gigaam` | [GigaAM](https://github.com/salute-developers/GigaAM) | [vd-gigaam/](vd-gigaam/) |
| `vd-whisper` | Whisper | TBD |

Queue / background runs: [`vd-srv`](../vd-srv/).

---

## Technology landscape

What matters for choosing and wiring models.

### Model families

| Family | Architecture | Strength | Cost / notes |
|--------|--------------|----------|--------------|
| **Whisper** (e.g. Large v3 Turbo) | Encoder–Decoder Transformer | Quality, 99 languages, punctuation | Heavier RTF; mel = Slaney, 128 bins, dynamic-range norm |
| **GigaAM** (CTC / RNNT / e2e) | Conformer + CTC or RNNT | Russian quality + speed | Small footprint; **HTK** mel, 64 bins, no center pad |
| **Parakeet TDT** | FastConformer + Token-Duration Transducer | English SOTA, fast decode | Poor Russian; custom LSTM + duration decoder |
| **Qwen3-ASR** | AuT encoder + LLM decoder | Multilingual / context-aware | Slowest on short clips; LLM decode cost |

Rough orientation (60 s Russian, Apple Silicon / Metal):

| Model | RTF | Cold start | Peak RAM |
|-------|-----|------------|----------|
| GigaAM v3 CTC | 0.017 | ~2.6 s | ~1.7 GB |
| Parakeet TDT v3 | 0.038 | ~5.9 s | ~4.7 GB |
| Whisper v3 Turbo | 0.110 | ~4.0 s | ~1.7 GB |
| Qwen3-ASR 0.6B | 0.114 | ~2.6 s | ~1.9 GB |

### Frontend is not interchangeable

Each family needs its own feature pipeline:

| Axis | Whisper | GigaAM | Parakeet | Qwen3-ASR |
|------|---------|--------|----------|-----------|
| Mel bins | 128 | 64 | 80 | 128 |
| n_fft | 400 | 512 | 512 | 400 |
| Mel scale | Slaney | **HTK** | From weights | Slaney |
| Log | log₁₀ | ln | ln | log₁₀ |
| Center pad | yes | **no** | yes | yes |
| Norm | dynamic range | none | per-utterance | dynamic range |

Wrong mel → garbage text with no useful error. That alone argues against one generic “preprocess then swap encoder” CLI.

### Decode paths differ

- **Whisper** — autoregressive decoder; temperature fallback when greedy looks bad.
- **GigaAM CTC** — greedy CTC (argmax → collapse blanks/dupes → SentencePiece); very fast.
- **Parakeet TDT** — joint token + duration; skips frames.
- **Qwen3-ASR** — LLM decode over projected audio embeddings.

### Runtime stack

Shared across transcription CLIs:

| Layer | Choice |
|-------|--------|
| Language / binary | **Rust** |
| ML backend | Candle (`candle-core`, `candle-nn`, …) + model-specific code in that crate |
| Weights | SafeTensors, GGUF, and/or converted `.ckpt` → tensors (per model) |
| Audio I/O | Rust crates (WAV); ffmpeg (or equiv.) for non-WAV before the model |
| Progress | stderr (`--progress=text\|json\|none`); stdout free |

Python (e.g. official `gigaam`) — **maintainer/CI only** (weight conversion, golden checks). Not shipped, not linked, not required at install or run time.

Quantization (GGUF Q8/Q4), VAD, diarization — only where that binary’s stack supports them. GigaAM CLI does not inherit Whisper diarization flags “because another binary has them”.

### Design principles

1. **One model ≠ all languages.** Multilingual Qwen can be weak on Russian names/punctuation; GigaAM/Whisper fit that niche better.
2. **Validate against a reference.** Golden / layer checks catch RoPE order, LSTM gates, STFT padding when porting to Rust.
3. **Ship only what the binary needs** — process boundaries instead of a shared plugin registry.

### Explicitly out of scope

- Single façade / trait registry over all families inside one process.
- One CLI with `--model-type whisper|gigaam|…` and a flat option bag.
- Python (PyTorch / `gigaam` / HF) as a runtime or install dependency for `vd-*` transcription binaries.

---

## How pieces fit

```
audio/video
    │
    ▼
┌─────────────┐     ┌──────────────┐
│  vd-gigaam    │     │  vd-whisper  │   … per-model CLIs
│  (GigaAM)   │     │  (Whisper)   │
└──────┬──────┘     └──────┬───────┘
       │                   │
       └─────────┬─────────┘
                 ▼
            vd-srv (queue)
```

Foreground: call the matching CLI directly.  
Background: `vd-srv` schedules jobs and invokes the right binary with that model’s flags.
