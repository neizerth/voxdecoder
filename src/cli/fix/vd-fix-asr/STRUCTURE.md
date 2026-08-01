# vd-fix-asr — project layout

Rust crate for the ASR wording-fix CLI.

**Status: implemented.** Workspace member: `src/cli/fix/vd-fix-asr`. Pack/`assets/` install remains a possible future.

Related: [README.md](README.md) (product notes) · [cli.md](cli.md) (flags) · [RUST.md](RUST.md) (fmt / clippy) · [TODO-languages.md](TODO-languages.md) · shared I/O: [`crates/`](../../../crates/)

---

## Philosophy

**Backend is an implementation detail.**

Same project rule as every `vd-fix-*` binary. Tomorrow the backend may be rules, Candle, ONNX Runtime, llama.cpp, a local HTTP daemon, or an ensemble — **none of that leaks** into `cli.md`, public modules, progress events, or dry-run JSON.

Exit code 4 stays backend-agnostic: *Inference backend failed to initialize*.

Naming: `asr/` — job name, never `engine/` / `model/` / `pipeline/`.

**Optional downloadable assets** (`install` / `remove` / `list` / `info`) are a *possible* future only if needed — same convention as `vd-gigaam` / `vd-fix-casing`, not part of the wording contract yet. When that happens, they live under `assets/`, not a folder named for one backend kind.

**Language priority:** `ru` = Russian with English insertions first. See [TODO-languages.md](TODO-languages.md).

---

## Non-goals

`vd-fix-asr` intentionally does **not**:

- infer missing transcript content
- summarize
- rewrite for readability / style
- normalize terminology to project-canonical forms (`vd-fix-terms`)
- translate
- use audio / re-run ASR
- restyle presentation (`vd-fix-casing`)
- invent information unsupported by the transcript, neighbors, or `--context`

---

## Tree (planned)

Crate lives at `src/cli/fix/vd-fix-asr/` in the repo:

```
src/cli/fix/vd-fix-asr/
├── Cargo.toml                  # TBD — add workspace member when implementing
├── README.md                   # product notes (this folder)
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── TODO-languages.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── types.rs                # re-export shared crates
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── run.rs
│   │   └── config_cmd.rs
│   │   # install/remove/list/info — possible future only
│   ├── config/
│   │   ├── mod.rs
│   │   ├── file.rs
│   │   └── resolve.rs
│   ├── assets/                 # possible future — catalog / pack / cache (not “the model”)
│   │   ├── mod.rs
│   │   ├── catalog.rs
│   │   └── pack.rs
│   ├── context/                # read-only — never &mut
│   │   ├── mod.rs
│   │   ├── neighbors.rs
│   │   ├── materials.rs        # --context: md / pdf / code / glossary / …
│   │   └── visit.rs            # visit_text_spans (neighbors + materials)
│   ├── paths.rs                # VD_FIX_ASR_* via `vd_artifact::paths`
│   └── asr/                    # this binary only — not a shared fix engine
│       ├── mod.rs
│       ├── fixer.rs            # AsrFixer::load / .fix → FixResult
│       ├── config.rs           # AsrLoadOptions
│       └── backend/            # private implementation detail
│
├── tests/                      # TBD
│   ├── unit/
│   └── e2e/
│
└── fixtures/
    ├── input/
    └── expected/
```

---

## Domain model

Shared types live in **`vd-artifact`** (plus `ProgressFormat` from `vd-progress`). This crate re-exports them via `types.rs`.

```rust
pub enum ArtifactType { /* txt, json, jsonl, srt, vtt, md, … */ }
pub enum Language { Ru, En, De, Auto /* … */ }
pub enum ProgressFormat { Text, Json }

/// Opaque span identity for neighbor lookup, progress, logging.
pub struct SpanId(/* … */);

/// Only handle the fixer may mutate. Timestamps / ids / metadata are unreachable.
pub struct TextSpan<'a> {
    pub id: SpanId,
    pub index: usize,
    pub text: &'a mut String,
}

/// Per-fix knobs. Keep empty until a real per-call dial appears
/// (e.g. strength / conservative / aggressive).
pub struct FixOptions {
    // reserved — prefer AsrLoadOptions for load-time config
}

pub struct FixResult {
    pub text: String,
    pub changed: bool,
}
```

### `AsrLoadOptions` (load-time — in `asr/config.rs`)

```rust
pub struct AsrLoadOptions {
    pub language: Language,
    /// Paths from repeatable `--context` (docs, glossaries, dictionaries, code, …).
    pub context_paths: Vec<PathBuf>,
    /// Neighboring segments window (CLI `--context-neighbors`).
    pub neighbor_window: u32,
}
```

Unlike `vd-fix-casing`, the fixer may **change words**. Guarantees are structural (spans / type / metadata), not lexical identity.

---

## Modules

| Path | Role |
|------|------|
| `vd-artifact` / `vd-output` / `vd-progress` | see [../../../crates/README.md](../../../crates/README.md) |
| `types.rs` | Re-export shared crate types |
| `cli/` | UX from [cli.md](cli.md) |
| `config/` | Persist + merge into `ResolvedRun` |
| `assets/` | Catalog / pack / cache (**possible future** — not backend-specific) |
| `context/` | Neighbors + `--context` materials — **read-only, never `&mut`**; `visit_text_spans` |
| `paths.rs` | `VD_FIX_ASR_*` via `vd_artifact::paths` |
| `asr/` | Wording rewriter (text + read-only context) |

---

## Assets (`assets/`) — possible future

Name is intentional: install / catalog / manifests / cache — **not** “the model”. Tomorrow the payload may be rules, one SafeTensors file, ORT, llama.cpp weights, or nothing downloadable.

Only if optional downloadable assets become necessary:

```text
{download_root}/
  ru/
    manifest.toml
    …pack assets…
```

Until then, prefer a builtin/default path with no `install` requirement. Env / flags for a download root stay reserved (`VD_FIX_ASR_MODELS_DIR`, `--download-root`) but are not part of the README product promise.

---

## Layer responsibilities

| Layer | Responsibility | Owns |
|-------|----------------|------|
| `assets/` | **packs** (future) | optional downloadable assets if ever needed |
| shared crates | **structure + paths** | `vd-artifact` + `vd-output` |
| `context/` | **hints** | neighbors + `--context` materials; **read-only forever** |
| `asr/` | **text** | wording repair inside the current span |
| `cli/` | **UX** | flags, dry-run, progress, exit codes |

### `context/` immutability

`context/` must **never** take or expose `&mut` to artifact text or materials. Neighbor windows and `--context` sources are hints only. The only mutable handle in the fix loop is the current `TextSpan::text`.

### Pipeline (planned)

```text
artifact::load              → typed artifact (know ArtifactType)
count / index text spans    → progress denominator + SpanId/index
output::path                → where to write (may depend on type)
context::load               → --context materials / neighbor policy (read-only)
artifact.visit_text_spans   → for each span + auto ctx
  asr::fixer.fix           → FixResult (words may change; this span only)
artifact::write             → same ArtifactType
```

Output path is resolved **after** load so extension / type-aware naming stays honest.

---

## Shared crates?

**Yes — lean on [`crates/`](../../../crates/).** Formats/spans → `vd-artifact`; `.fixed.` → `vd-output`; progress → `vd-progress`. Do not copy those modules back into the CLI crate.

| Keep in shared crates | Keep in this binary |
|------------------------|---------------------|
| Artifact (`vd-artifact`) / progress (`vd-progress`) / `.fixed.` (`vd-output`) / `paths` | `context/` (`visit_text_spans`, materials) |
| `ArtifactType`, `TextSpan { id, index, text }`, … | `asr/` backend; future asset-pack install UX |

| Rejected |
|----------|
| Shared presentation / ASR / terms engine |
| `FixModel` trait |

Naming `src/asr/` (not `engine/`) is intentional.

---

## Public fix API (planned)

```rust
let fixer = AsrFixer::load(AsrLoadOptions {
    language: Language::Ru,
    context_paths,
    neighbor_window: 1,
})?;  // Err only on real backend init failure → exit 4

artifact.visit_text_spans(|span, ctx| {
    // ctx: read-only neighbors + materials; never &mut
    let result: FixResult = fixer.fix(span, ctx, FixOptions::default())?;
    if result.changed {
        *span.text = result.text;
    }
    Ok(())
})?;

write(&artifact, &output_path)?;
```

`visit_text_spans` (or `for_each_text_span`) owns neighbor wiring internally so call sites stay short. Dry-run / CLI surface stays as in [cli.md](cli.md).

---

## Guarantees in code

| Layer | Responsibility | Enforces |
|-------|----------------|----------|
| `assets/` | packs (future) | optional; corrupt/required load → exit 4 |
| `vd-artifact` | structure | Only `TextSpan::text` is mutable; type preserved on write |
| `context/` | hints | **No `&mut` ever**; never mutates other spans |
| `asr/` | text | Wording only — see Non-goals |
| `vd-output` | filesystem | `.fixed.{{ext}}`; `-o` XOR `-d` XOR `--in-place` |
| `cli/` | UX | Exit codes from [cli.md](cli.md); dry-run never writes |

---

## Progress

| Command | Events |
|---------|--------|
| `install` (future) | `start` → `downloading` → `done` / `error` |
| `run` | `start` → `loading` → `processing` (`percent`, `span`, `span_total`) → `writing` → `done` / `error` |

Keep **both** `percent` and `span` / `span_total`. Span counters are the useful signal for multi-span artifacts; percent remains for UIs that only show a bar.

Same `--progress=text\|json` and `-q` as [cli.md](cli.md).

---

## Tests and fixtures

All tests under `tests/` — **no** `#[cfg(test)]` in `src/`.

| Path | Role |
|------|------|
| `tests/unit/cli.rs` | clap / conflicts |
| `tests/unit/context.rs` | read-only; materials load; neighbor window |
| `tests/unit/asr.rs` | ru+en-insertions fixtures; structural guarantees; non-goals |
| `tests/e2e/binary.rs` | progress `span`/`span_total`, dry-run, `--context` |
| `src/crates/*/tests/unit/` | artifact detect / spans / roundtrip / output paths |

```bash
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-fix-asr --test unit
cargo test -p vd-fix-asr --test e2e
```

---

## Build

```bash
cd src/cli/fix/vd-fix-asr
cargo build --release
cargo test
cargo run -- run -i fixtures/input/sample.txt --dry-run
```

Binary name: `vd-fix-asr`.  
Workspace member: `src/cli/fix/vd-fix-asr` (depends on `vd-artifact` / `vd-output` / `vd-progress`).

---

## Public contract note

Model family, inference runtime, and backend implementation are intentionally **outside** the public CLI contract.
