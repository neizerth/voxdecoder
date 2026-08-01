# vd-diarize — project layout

Rust crate: **local-first diarization** — standalone CLI **and** binder for `use: diarize` on the shared Executor.

**Status: implemented.** Workspace member: `src/cli/process/vd-diarize`.

Default backend for CI: `stub`. Heavy providers (`pyannote`, `nemo`) install assets; inference runtime lands later.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md) · [../vd-pipeline/](../vd-pipeline/) · [../vd-meeting/](../vd-meeting/)

---

## Philosophy

```text
audio  →  vd-diarize (CLI ≡ capability diarize)  →  SpeakerTimeline
```

- **One question:** anonymous who-spoke-when for **one** recording.
- **Local-first:** inference always local; assets may download once, cache, reuse; **no audio is transmitted**.
- **Stable artifact, swappable backends:** pyannote / NeMo / … change under `backend:`; Job contract stays `SpeakerTimeline`.
- **Speaker ids are artifact-local** — `S0 → Alice` is `meeting-merge`, not this crate.
- **Never** ASR, fix-*, Meeting Model, or multi-track merge.

Composition with the rest of process (keep this boundary in code and docs):

```text
Meeting (vd-meeting Planner)
        ↓
      Job
        ↓
 capability: diarize  (vd-diarize)
        ↓
  SpeakerTimeline
        ↓
 meeting-merge capability
```

`vd-diarize` knows nothing about Meeting. `vd-meeting` knows nothing about pyannote internals.

Product: [README.md](README.md).

---

## Non-goals

- Transcription / wording / terminology
- Global speaker identity across files
- Cloud upload of audio
- Promising a single permanent backend
- Owning Meeting Model / `meeting-merge`
- WhisperX-as-diarizer (Whisper + pyannote wrapper)

---

## Tree (target)

```
src/cli/process/vd-diarize/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs                  # run_diarize() for Executor binder / tests
│   ├── paths.rs
│   ├── cli/                    # run / install / remove / list / info / config
│   ├── config/
│   ├── artifact/               # SpeakerTimeline schema I/O (canonical artifact)
│   ├── backend/                # providers (local inference)
│   │   ├── mod.rs              # Backend trait + resolve
│   │   ├── pyannote/           # or pyannote.rs
│   │   └── nemo/
│   ├── assets/                 # install / cache / resolve backend-specific assets
│   └── status/
│
└── tests/
    ├── unit/                   # SpeakerTimeline schema, id locality, backend options
    ├── integration/            # stub backends → artifact shape
    ├── e2e/                    # binary; real backend gated
    └── fixtures/
        ├── audio/
        └── artifacts/          # golden SpeakerTimeline
```

Executor binding: `vd-pipeline` invokes this crate’s library (or subprocess CLI) for `use: diarize` — same implementation as `vd-diarize run`.

---

## Canonical artifact: SpeakerTimeline

Logical entity is always **SpeakerTimeline** (not “a json file”).

Exports may vary (`*.json`, msgpack, …); the type does not:

```text
ArtifactType::SpeakerTimeline
```

Describes **speech activity only** — never transcript text. See [README.md](README.md#what-is-a-diarization-artifact).

---

## Domain model

```rust
/// Canonical diarization result.
pub struct SpeakerTimeline {
    pub version: u32,
    pub audio: AudioRef,
    pub speakers: Vec<SpeakerId>,       // S0, S1, … — local to this artifact
    pub segments: Vec<Segment>,
    pub overlaps: Vec<Overlap>,         // optional / empty
    pub embeddings: Option<Embeddings>, // structured — not a binary blob
    pub speech_regions: Vec<Region>,    // optional / empty
    pub backend: BackendInfo,           // required — for debug / repro
}

pub struct Embeddings {
    pub per_speaker: Vec<SpeakerEmbedding>,
    // later: per_segment, multiple models, …
}

pub struct SpeakerEmbedding {
    pub speaker: SpeakerId,
    pub model: String,
    pub vector: Vec<f32>,
}

pub struct BackendInfo {
    pub provider: String,   // pyannote | nemo | …
    pub model: String,      // e.g. speaker-diarization-3.1, sortformer
    pub version: Option<String>,
    pub device: Option<String>,
}

pub struct BackendSpec {
    pub provider: String,
    pub model: Option<String>,
}

pub struct DiarizeRequest {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub backend: BackendSpec,           // not "family"
    pub options: BTreeMap<String, ArgValue>,
}
```

Job / CLI shape:

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

## Local model assets

Assets are **backend-specific**. `install` means **install assets**, not merely “download one checkpoint”.

They may include:

- segmentation models
- speaker embedding models
- clustering models / rules
- tokenizers / configs
- other runtime files the provider needs

Assets are cached locally and shared between CLI runs (and Executor invocations on the same machine).

Sources: Hugging Face, local directory, enterprise mirror, bundled packs — distribution channels only; inference stays local.

---

## Modules

| Path | Role |
|------|------|
| `cli/` | `run` + asset commands (`install` / `remove` / `list` / `info`) + config |
| `artifact/` | Read/write **SpeakerTimeline**; validate no text |
| `backend/` | Providers behind a common trait (`pyannote/`, `nemo/`, …) |
| `assets/` | Resolve / install / cache backend assets |
| `config/` | Default `backend.provider` / `model`, device, asset roots |
| `status/` | Progress |

---

## Runtime

```text
resolve input audio
      ↓
resolve backend          (CLI / Job options / config → BackendSpec)
      ↓
resolve assets           (installed cache; fail with install hint if missing)
      ↓
load backend             (provider + weights into local runtime)
      ↓
infer locally            → SpeakerTimeline
      ↓
write artifact           (register path for Executor)
```

Local-first:

```text
Inference is always local.

Model assets may be downloaded once,
cached locally,
and reused.

No audio is transmitted.
```

---

## Tests

| Layer | Must prove |
|-------|------------|
| Unit | SpeakerTimeline schema; required `BackendInfo`; no text; typed embeddings |
| Integration | Stub backend → valid artifact; switch `provider` / `model` via options |
| E2E | `run -i …`; `install`/`list` assets; full backend gated (`VD_DIARIZE_E2E_FULL=1`) |

```bash
# once crate exists:
cargo test -p vd-diarize
./scripts/test.sh vd-diarize
```

---

## Public contract note

**One audio → SpeakerTimeline (local inference).**  
CLI and `use: diarize` share the implementation. Backend provider/model are swappable. Identity resolution and Meeting knowledge are out of scope.
