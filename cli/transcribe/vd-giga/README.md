# vd-giga — GigaAM specifics

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI signature: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md).

`vd-giga` is a **Rust** CLI for [GigaAM](https://github.com/salute-developers/GigaAM) only: Conformer + CTC/RNNT in-process via Candle (GigaAM-specific mel / RoPE / decode). **Zero Python at runtime** — binary + converted weights. Options mirror load/transcribe knobs (`-m`, `--device`, `--flash`, …); no Whisper/Parakeet/Qwen flags.

---

## Why its own binary

GigaAM is not “Whisper with a smaller checkpoint”:

| Topic | GigaAM | Why a shared adapter fails |
|-------|--------|----------------------------|
| Mel | 64 bins, **HTK**, ln, **no** center pad, no DR norm | Whisper Slaney/128/center — wrong mel → garbage RU text |
| Encoder | Conformer (Macaron: FFN → MHSA+RoPE → depthwise conv → FFN) | RoPE applied **before** Q/K projections (non-standard) |
| Decode (CTC line) | Greedy CTC + SentencePiece | No Whisper temperature fallback / beam |
| Language niche | Strong Russian, tiny RTF | “Multilingual” defaults from other families hurt RU |
| Weights | Catalog `.ckpt` / `.pt` + `download_root` | Not the same as HF Whisper dirs / GGUF-only flows |

So `-m`, `--device`, `--flash`, `--no-fp16-encoder`, `--download-root`, `--word-timestamps` stay first-class here — and nowhere else.

---

## Variants we care about

Catalog names (CLI `-m` / aliases): see [cli.md](cli.md).

| Line | Role |
|------|------|
| `v*_ctc`, `v*_e2e_ctc` | Conformer + CTC — fast path (RTF can be ~0.017 on CTC) |
| `v*_rnnt`, `v*_e2e_rnnt` | RNNT / e2e RNNT — often better quality, different decode cost |
| SSL / emo | Out of scope for transcription CLI |

Default in CLI: `v2_rnnt` (override via config). Prefer `v3_e2e_*` when quality matters; CTC when latency/RAM matter.

On Russian conversational audio, GigaAM CTC is typically close to Whisper on sense, with small artifacts, and much stronger than generic multilingual small models.

---

## Audio / features (do not “genericize”)

If we ever bypass the official preprocessor, match GigaAM training front-end:

| Parameter | Value |
|-----------|-------|
| Sample rate | 16 kHz mono |
| Mel bins | 64 |
| n_fft | 512 |
| hop_length | 160 |
| Mel scale | **HTK** (`f_mel = 2595 * log10(1 + f_hz / 700)`), not Slaney |
| Log | natural log (`ln`) |
| Center padding in STFT | **off** |
| Normalization | none (not Whisper dynamic-range) |

Whisper-style Slaney mel is a common root cause of garbled GigaAM output. Implement HTK mel (and the rest of this table) in the Rust audio path; do not reuse a Whisper feature extractor.

Long audio: official `transcribe_longform` pulls **pyannote / HF** — out of scope for `vd-giga` (no longform / diarize / HF in [cli.md](cli.md)). Chunk or truncate at our boundary, or leave long-form to another tool.

---

## Load / inference knobs

CLI flags map to the same ideas as the reference Python API (`load_model` / `transcribe`), implemented in Rust:

| CLI | Meaning | Default |
|-----|---------|---------|
| `-m` | Catalog name or local `.ckpt`/`.pt` (converted / loaded into Candle) | `v2_rnnt` |
| `--device` | `cpu` / `cuda` / `auto` (Metal where available) | `auto` |
| `--no-fp16-encoder` | Disable FP16 encoder | FP16 on |
| `--flash` | Enable FlashAttention when supported | off |
| `--download-root` | Checkpoint directory | managed cache |
| `--word-timestamps` | Word-level timestamps | off; only with `--format json` or `--segments` |

CTC path is naturally greedy and cheap — good default for batch RU jobs. RNNT/e2e may need more VRAM/time; expose via `-m`, don’t invent a second “quality” enum.

---

## Implementation pitfalls

When porting GigaAM to Rust / Candle (validate against a Python reference):

1. **RoPE order** — rotate embeddings **then** project Q/K. Standard “project then RoPE” diverges silently; catch with layer-by-layer golden tensors.
2. **CTC collapse** — blank + repeat removal before detokenize; wrong blank id → empty/garbled text.
3. **Checkpoint layout** — catalog downloads into `download_root` as `{name}.ckpt`; convert once to SafeTensors (or load with an explicit mapping). Local finetunes may be `.pt` with a different stem — resolve name vs path the way [cli.md](cli.md) describes.
4. **STFT padding** — GigaAM expects no center pad; Whisper-style reflect center pad will skew edges.
5. **Don’t mix mel filters from weights of another family** (Parakeet stores filters in the checkpoint; GigaAM does not — don’t copy that path).

Production path: Rust binary + published weights only. Python is **not** a `vd-giga` dependency — use it only in maintainer/CI scripts for checkpoint conversion and golden dumps, then drop it from the release path.

---

## Performance expectations

Rough orientation (Metal / Apple Silicon, ~60 s RU):

- RTF ≈ **0.017** (CTC) — often several times faster than Whisper Turbo.
- Peak RAM ≈ **1.7 GB** — good “minimal RAM” pick for Russian.
- Cold start a few seconds — still small vs multi-LLM ASR.

Exact numbers depend on device (`cpu` vs `cuda`/`mps`), FP16, flash, and variant (CTC vs RNNT). `--dry-run` should show the resolved load plan before paying cold start.

---

## Boundaries (what vd-giga is not)

| Not in vd-giga | Where it lives |
|----------------|----------------|
| Whisper / Parakeet / Qwen | Other CLIs |
| Diarization, HF token, pyannote | e.g. `vd-dia-*` / other tools |
| Job queue / multi-run state | `vd-srv` |
| Universal `--model-type` | Never — wrong product shape |

Progress and results: stderr `--progress`, files via `-o`/`-d`/`--segments` — see [cli.md](cli.md).
