# vd-preprocess — project layout

Rust crate: **universal media filter-chain executor** — domain library **and** CLI surface for `use: preprocess` on the shared Executor.

**Status: implemented.** Path: `src/cli/process/vd-preprocess`. Default CI provider: `stub`; production default often `ffmpeg`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md) · [../vd-pipeline/](../vd-pipeline/) · [../vd-postprocess/](../vd-postprocess/)

---

## Philosophy

```text
Media
  +
Filter chain
  +
Provider(s)
  ↓
Prepared Media
```

Not “a pile of CLI flags”. A filter is a **typed operation** on a **provider**; the chain is the product contract — same idea as recipes in [`vd-postprocess`](../vd-postprocess/).

- **No empty chain** — zero filters is an error (builders supply defaults; the binary does not invent them).
- **Filters are user / builder assets** — CLI never hard-codes “meeting mode” vs “podcast mode”.
- **Provider + operation** — ffmpeg today; sox / deepfilternet / rnnoise / demucs later without Job schema churn.
- **`type: X` sugar** — expands to default provider + `operation: X`.
- **Normal DAG capability** — per-branch preprocess is first-class (meeting tracks vs room).
- **CLI ≡ capability** — same request shape; binder may use library or binary.
- **Default placement** — Job builders put preprocess **before** ASR / diarize when preparing media; not a special Executor rule.

Product: [README.md](README.md).

---

## Unified capability contract

Every capability in VoxDecoder shares one shape:

```text
Inputs + Options  →  Capability  →  Artifacts
```

| Capability | In | Out |
|------------|----|-----|
| **`preprocess`** | media + filters + provider(s) | prepared media |
| `transcribe` | audio | transcript |
| `fix-*` | transcript | transcript |
| `diarize` | audio | timeline |
| `meeting-merge` | tracks + timeline + model | meeting |
| `postprocess` | artifacts + recipes + provider | artifacts |

`vd-pipeline` / `vd-srv` / MCP stay universal: they never learn “trim silence” vs “Demucs” — only artifacts and options.

---

## Non-goals

- Flat flag-only product API as the source of truth (`--speed` / `--normalize` without a chain)
- Silent built-in filter chain when none given
- Assuming every provider is ffmpeg
- Rewriting time on timeline-sensitive branches without planner intent
- Owning ASR / diarize / Meeting Model / postprocess recipes
- Coupling the binder to “must spawn a subprocess”
- Cloud upload of user media by default

---

## Tree (target)

Domain logic lives under `preprocess/` (mirror `vd-postprocess/postprocess/`):

```
src/cli/process/vd-preprocess/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs                      # plan / execute for MCP, pipeline binder, tests
│   ├── paths.rs
│   ├── cli/                        # flags → PreprocessRequest (thin)
│   ├── config/
│   ├── status/
│   └── preprocess/                 # domain
│       ├── mod.rs
│       ├── executor.rs             # plan + execute (dry-run aware)
│       ├── filter.rs               # FilterSpec · groups · sugar expand
│       ├── chain.rs                # ordered FilterChain validate
│       ├── result.rs               # PreprocessResult / PreparedMedia
│       └── provider.rs             # MediaProvider trait + backends
│
└── tests/
    ├── unit/
    ├── integration/
    ├── e2e/
    └── fixtures/
        ├── media/
        ├── chains/                 # example chains for tests — not product builtins
        └── …
```

| Path | Role |
|------|------|
| `cli/` | UX only → `PreprocessRequest` |
| `preprocess/executor.rs` | Resolve providers → expand sugar → validate chain → plan → execute |
| `preprocess/filter.rs` | `FilterSpec` + catalog metadata (group, operation) |
| `preprocess/chain.rs` | Ordered list; reject empty |
| `preprocess/result.rs` | Prepared media artifact path(s) |
| `preprocess/provider.rs` | `MediaProvider` (+ ffmpeg / sox / deepfilternet / …) |
| `config/` | Default provider / progress / binary paths |

---

## Domain model (planned)

```rust
/// Library / Job request.
pub struct PreprocessRequest {
    pub input: ArtifactRef,                 // media in
    /// Ordered filter chain (must be non-empty after expand).
    pub filters: Vec<FilterSpec>,
    /// Default provider for `type:` sugar.
    pub provider: Option<String>,           // e.g. "ffmpeg"
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub overwrite: bool,
}

/// One step in the chain.
pub struct FilterSpec {
    pub provider: String,                   // ffmpeg | sox | deepfilternet | …
    pub operation: String,                  // normalize | resample | speed | …
    pub params: BTreeMap<String, ArgValue>, // rate, factor, min_duration, …
}

/// GUI / docs catalog — not required at runtime for unknown ops.
pub enum FilterGroup {
    Media,
    Audio,
    Timing,
    Channels,
}

pub struct ExecutionPlan {
    pub default_provider: String,
    pub steps: Vec<PlannedFilter>,          // resolved binary/model + params + temp paths
}

pub struct PreparedMedia {
    pub id: Option<String>,
    pub path: PathBuf,
}

pub struct PreprocessResult {
    pub output: PreparedMedia,
    /// Optional sidecars (e.g. split channels) — registered when declared.
    pub extras: Vec<PreparedMedia>,
}
```

### YAML shapes

Long form (explicit provider):

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

Short form (`type` → default provider + operation):

```yaml
provider: ffmpeg
filters:
  - type: trim-silence
    min_duration: 500ms
  - type: normalize
  - type: speed
    factor: 1.15
```

Empty `filters` → usage / exit 2.

---

## Filter catalog (initial)

| Group | Operations (examples) | Typical provider |
|-------|----------------------|------------------|
| **Media** | `extract-audio`, `convert`, `resample`, `mono`, `stereo` | ffmpeg |
| **Audio** | `normalize`, `denoise`, `highpass`, `lowpass`, `compressor` | ffmpeg / deepfilternet / rnnoise |
| **Timing** | `speed`, `trim-silence`, `trim`, `chunk` | ffmpeg |
| **Channels** | `split-channels`, `merge-channels` | ffmpeg |

Params are operation-specific (`rate`, `factor`, `min_duration`, `cutoff_hz`, …). Unknown `provider` / `operation` → plan-time error (or provider-reported unsupported).

---

## Capability wiring (`vd-pipeline`)

```text
Capability::Preprocess
        ↓
  Backend binding          # library call and/or CLI — binder detail
        ↓
  Artifacts (PreparedMedia)
```

Do **not** document the product as “spawn `vd-preprocess`”. Subprocess is one possible binding; in-process library is another. Job authors only see `use: preprocess`.

| Phase | Behavior |
|-------|----------|
| Schema | `Capability::Preprocess` |
| Resolve | Non-empty filters; expand `type:` sugar; map input → `ArtifactRef` |
| Bind | Backend binding → `preprocess::executor` |
| Outputs | Register prepared media under step `id` |
| Progress | Standard step events (per-filter phase optional) |

Default Job builder (`vd-pipeline run -i …` without explicit Job):

```text
preprocess (trim-silence / normalize / …) → transcribe → …
```

Meeting planner: **per-branch** preprocess nodes with role-appropriate chains.

Until bound: ~~reserved~~ — **implemented** (`Capability::Preprocess` + subprocess binder).

```text
track.wav ──► preprocess ──► transcribe
room.wav  ──► preprocess ──► diarize
```

---

## Algorithm

Dry-run must print a full **ExecutionPlan** without invoking DSP:

```text
collect request (input, filters, default provider, output options)
      ↓
reject if no filters
      ↓
expand type: sugar → provider + operation
      ↓
resolve MediaProvider(s) for each distinct provider
      ↓
validate operations / params against provider capabilities
      ↓
build ExecutionPlan          # temp paths, argv / model hooks
      ↓
[--dry-run → emit plan → stop]
      ↓
execute plan left-to-right
      ↓
write final PreparedMedia
      ↓
return PreprocessResult
```

Order: **expand → resolve providers → validate → plan → execute**.

---

## Tests (planned)

| Topic | Proof |
|-------|--------|
| no filters | exit 2 |
| type sugar | `type: normalize` → `provider: ffmpeg`, `operation: normalize` |
| chain order | plan lists filters in YAML order |
| ExecutionPlan dry-run | plan JSON; no ffmpeg invoke |
| provider stub | CI without system ffmpeg |
| binder | `Capability::Preprocess` → backend binding |
| default Job | builder inserts preprocess before transcribe |
| meeting branches | two preprocess steps, different chains |

```bash
# once crate exists:
cargo test -p vd-preprocess
./scripts/test.sh vd-preprocess
```

---

## Public contract note

**Filter chain + MediaProvider(s) + media input → Prepared Media.**  
Same Inputs → Capability → Artifacts contract as the rest of the platform. Capability name is `preprocess`, not “ffmpeg” or “denoise”.
