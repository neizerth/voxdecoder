# vd-giga — project layout

Rust + Candle crate for the GigaAM CLI. **No Python at runtime.**

Related: [README.md](README.md) (model notes) · [cli.md](cli.md) (flags)

---

## Tree

Crate lives at `cli/vd-giga/` in the repo:

```
cli/vd-giga/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── run.rs
│   │   ├── config_cmd.rs
│   │   ├── install.rs
│   │   ├── remove.rs
│   │   ├── list.rs
│   │   └── info.rs
│   ├── config/                 # load / save / merge / defaults
│   │   ├── mod.rs
│   │   ├── file.rs
│   │   └── resolve.rs          # CLI > config > default → ResolvedConfig
│   ├── output/
│   │   ├── mod.rs
│   │   ├── path.rs             # -o XOR -d, --overwrite, segments path
│   │   ├── writer.rs
│   │   └── formats.rs          # txt | json | srt | vtt
│   ├── progress.rs             # --progress text|json|none → stderr
│   ├── paths.rs                # models root, VD_GIGA_MODELS_DIR, config path
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── decode.rs           # container → PCM (ffmpeg / hound / …)
│   │   └── resample.rs         # → 16 kHz mono
│   └── gigaam/                 # this model only — not a shared ASR engine
│       ├── mod.rs
│       ├── model.rs            # GigaModel::load / .transcribe
│       ├── config.rs           # variant, device, fp16, flash, …
│       ├── weights/
│       │   ├── mod.rs
│       │   ├── load.rs         # cache / download / deserialize
│       │   └── mapping.rs      # ckpt / safetensors name map
│       ├── frontend/
│       │   └── mel.rs          # HTK, 64 bins, ln, no center pad
│       ├── encoder/
│       │   └── conformer.rs    # includes RoPE (before Q/K) unless it grows large
│       └── decoder/
│           ├── ctc.rs
│           └── rnnt.rs         # when supported
│
├── tests/
│   ├── cli.rs
│   ├── output_paths.rs
│   └── golden_ctc.rs
│
├── fixtures/                   # wav / json / golden tensors (not scripts/)
│   ├── audio/
│   ├── expected/
│   └── golden/
│
├── scripts/                    # maintainer / CI only
│   ├── README.md
│   ├── convert_ckpt.py
│   ├── dump_golden.py
│   └── requirements.txt
│
└── models/                     # local weights (gitignored)
    └── *.safetensors
```

Docs for this binary: `cli/transcribe/vd-giga/` (this folder).

---

## Modules

| Path | Role |
|------|------|
| `cli/` | Commands from [cli.md](cli.md) |
| `config/` | Persist + merge into `ResolvedConfig` |
| `output/` | Paths, writers, format serializers |
| `progress.rs` | stderr progress; stdout free |
| `paths.rs` | Platform config + managed model dir |
| `audio/` | Decode + resample for GigaAM |
| `gigaam/` | Candle: mel → Conformer → CTC/RNNT → text |

`paths.rs` and `progress.rs` stay flat — small, stable. `progress` is the most likely first extract if a second CLI appears.

---

## Shared crates?

**Default: none.** Keep helpers inside `vd-giga`.

Extract a shared crate only when a second CLI is real **and** the duplicated code is identical (not “almost”).

| Tempting to share | Reality |
|-------------------|---------|
| Mel / feature extract | **Different per model** — must not share |
| Conformer / Whisper encoder | Model-specific — must not share |
| `--progress` NDJSON | Convention; optional late extract |
| `-i` / `-o` / `-d` / `--overwrite` | Similar UX; duplicate until it hurts |
| WAV / ffmpeg glue | Possible later; start in `audio/` |
| `AsrModel` trait / engine | **Rejected** |

Even after an extract: helpers only (`progress`, maybe `audio` decode), never a multi-model façade.

Naming `src/gigaam/` (not `engine/` / `asr/`) is intentional: it is **this model’s** stack. A future `vd-whisper` would get `src/whisper/`, not a shared engine folder.

---

## `gigaam/` layout

Grouped by role so the tree stays readable past ~15 files:

```
gigaam/
├── model.rs      # public API
├── config.rs
├── weights/
├── frontend/     # mel
├── encoder/      # conformer (+ RoPE inline)
└── decoder/      # ctc, rnnt
```

RoPE lives **inside** `encoder/conformer.rs` unless it grows large (~200+ lines) — then split `rope.rs` next to it.

---

## Public inference API

```rust
let model = GigaModel::load(GigaLoadOptions {
    model: "v2_rnnt".into(),
    device: Device::Auto,
    fp16_encoder: true,
    flash: false,
    download_root: None,
})?;

let out = model.transcribe(&samples, TranscribeOptions {
    word_timestamps: false,
})?;
```

Not free functions `load` / `transcribe` — method style matches Rust usage and keeps state on `GigaModel`.

---

## Tests and fixtures

| Path | Role |
|------|------|
| `tests/cli.rs` | clap / exit codes / flag conflicts |
| `tests/output_paths.rs` | `-o` / `-d` / `--segments` / `--overwrite` |
| `tests/golden_ctc.rs` | layer or end-to-end vs golden tensors/text |
| `fixtures/audio/` | short wavs |
| `fixtures/expected/` | expected transcripts / JSON |
| `fixtures/golden/` | dumped tensors from `scripts/dump_golden.py` |

Keep fixtures out of `scripts/` — scripts generate or convert; fixtures are committed inputs for `cargo test`.

---

## `scripts/`

Python **only** here: checkpoint → SafeTensors/GGUF, golden dumps. Not linked into the binary. Users install `vd-giga` + published weights.

---

## Build

```bash
cd cli/vd-giga
cargo build --release
cargo test
cargo run -- run -i sample.wav --dry-run
cargo run -- install v2_rnnt --progress=json
```

Binary name: `vd-giga`.
