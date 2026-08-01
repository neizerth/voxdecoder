# vd-fix-casing — project layout

Rust crate for the presentation-fix CLI.

Related: [README.md](README.md) (product notes) · [cli.md](cli.md) (flags) · [RUST.md](RUST.md) (fmt / clippy) · [TODO-languages.md](TODO-languages.md)

---

## Philosophy

**Backend is an implementation detail.**

This is a project rule for every `vd-fix-*` binary, not only casing. Tomorrow the backend may be Candle, ONNX Runtime, llama.cpp, regex, a rule engine, or an ensemble — **none of that leaks** into `cli.md`, public modules, progress events, or dry-run JSON.

Exit code 4 stays backend-agnostic: *Inference backend failed to initialize* (e.g. corrupt pack). Missing pack is **not** exit 4 when a builtin lexicon exists.

Same idea for naming: `casing/` / future `asr/` / `terms/` — job names, never `engine/` / `model/` / `pipeline/`.

**Installable packs** follow the `vd-gigaam` UX (`install` / `remove` / `list` / `info`) but are **optional** for the rules backend: `run` uses an embedded lexicon when no pack is installed; an installed pack overrides it.

---

## Tree

Crate lives at `cli/fix/vd-fix-casing/` in the repo:

```
cli/fix/vd-fix-casing/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── types.rs                # domain model (see below)
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── run.rs
│   │   ├── install.rs
│   │   ├── remove.rs
│   │   ├── list.rs
│   │   ├── info.rs
│   │   └── config_cmd.rs
│   ├── config/                 # load / save / merge / defaults
│   │   ├── mod.rs
│   │   ├── file.rs
│   │   └── resolve.rs          # CLI > config > default → ResolvedRun
│   ├── models/                 # catalog + installable language packs
│   │   ├── mod.rs
│   │   ├── catalog.rs          # ru / en (shipping), de (reserved)
│   │   └── pack.rs             # install / remove / lexicon load
│   ├── artifact/               # mutable text-span iterator over artifacts
│   │   ├── mod.rs
│   │   ├── detect.rs           # extension / sniff → ArtifactType
│   │   ├── load.rs
│   │   ├── writer.rs           # serialize same type
│   │   ├── text_spans.rs       # apply_to_text_spans / count_text_spans
│   │   └── formats/
│   │       ├── mod.rs
│   │       ├── txt.rs
│   │       ├── json.rs         # json + jsonl string fields
│   │       ├── srt.rs
│   │       ├── vtt.rs
│   │       └── md.rs
│   ├── output/
│   │   ├── mod.rs
│   │   └── path.rs             # -o XOR -d XOR --in-place, .fixed., --overwrite
│   ├── progress.rs             # --progress text|json → stderr; -q disables
│   ├── paths.rs                # config + models dir, env overrides
│   └── casing/                 # this binary only — not a shared fix engine
│       ├── mod.rs
│       ├── fixer.rs            # CasingFixer::load / .fix → FixResult
│       ├── config.rs           # language + models_dir
│       └── backend/            # private implementation detail
│           ├── mod.rs
│           ├── tokens.rs
│           ├── restore.rs
│           └── normalize.rs
│
├── tests/
│   ├── unit/
│   │   ├── mod.rs
│   │   ├── cli.rs
│   │   ├── output_paths.rs
│   │   ├── artifact_detect.rs
│   │   ├── artifact_text_spans.rs
│   │   ├── artifact_roundtrip.rs
│   │   └── casing.rs
│   └── e2e/
│       ├── mod.rs
│       └── binary.rs           # install optional, builtin without pack, progress %
│
└── fixtures/
    ├── input/
    └── expected/
```

Docs for this binary: `cli/fix/vd-fix-casing/` (this folder).

---

## Domain model (`types.rs`)

Core types live in one place so modules do not grow cyclic deps or “types scattered at random”:

```rust
pub enum ArtifactType { /* txt, json, jsonl, srt, vtt, md, … */ }
pub enum Language { Ru, En, De, Auto /* … */ }
pub enum ProgressFormat { Text, Json }

pub struct FixOptions { /* reserved; keep thin */ }

/// Only handle the fixer may mutate. Timestamps / ids / metadata are unreachable.
pub struct TextSpan<'a> {
    pub text: &'a mut String,
}

pub struct FixResult {
    pub text: String,
    pub changed: bool,
}
```

`TextSpan` is the strongest guarantee in the architecture: `casing` never sees raw artifact JSON/SRT — only spans.

---

## Modules

| Path | Role |
|------|------|
| `types.rs` | Domain model: `ArtifactType`, `Language`, `TextSpan`, `FixResult`, … |
| `cli/` | UX from [cli.md](cli.md): `run`, `install`, `remove`, `list`, `info`, `config` |
| `config/` | Persist + merge into `ResolvedRun` |
| `models/` | Catalog + pack install / lexicon load |
| `artifact/` | Find mutable text spans; load/write **same type** |
| `output/` | Filesystem paths only |
| `progress.rs` | stderr progress (`downloading` / `processing` %); stdout free |
| `paths.rs` | Config + platform cache models dir (`VD_FIX_CASING_*`) |
| `casing/` | Presentation rewriter (text only) |

---

## Language packs (`models/`)

Shipping catalog: **`ru`**, **`en`**. Layout under models dir:

```text
{download_root}/
  ru/
    manifest.toml
    lexicon.json
  en/
    …
```

- `install` writes the pack (embedded shipping assets today; remote weights can plug in later).
- Progress: `downloading` with `percent` + `bytes_done` / `bytes_total`.
- `CasingFixer::load` **requires** an installed pack → exit **4** if missing.
- Pack content is an implementation detail (`backend = "rules"` in manifest). Future ONNX/Candle files land in the same directory without changing CLI.

Env / flags: `VD_FIX_CASING_MODELS_DIR`, `--download-root`, `config set download_root`.

---

## Layer responsibilities

| Layer | Responsibility | Owns |
|-------|----------------|------|
| `models/` | **packs** | install, lexicon, “is installed?” |
| `artifact/` | **structure** | detect, load, `TextSpan` iteration, write |
| `casing/` | **text** | presentation rewrite inside a span |
| `output/` | **filesystem** | resolve output path (`.fixed.`, `-o`/`-d`/`--in-place`) |
| `cli/` | **UX** | flags, dry-run, progress, exit codes |

Pipeline:

```text
models::resolve_lexicon → installed pack or builtin
output::path           → where to write
artifact::load         → typed artifact in memory
count_text_spans       → progress denominator
apply_to_text_spans    → TextSpan<'_>
casing::fixer.fix    → FixResult
artifact::write        → same ArtifactType on disk
```

---

## Shared crates?

**Default: none.** Keep helpers inside `vd-fix-casing`.

**Duplicate until it hurts.** Three copies are cheaper than one wrong abstraction.

| Tempting to share | Reality |
|-------------------|---------|
| Artifact / progress / `.fixed.` paths | Similar across `vd-fix-*`; duplicate until it hurts |
| Language-pack install UX | Same *convention* as `vd-gigaam`; keep local |
| Presentation / ASR / terms backends | **Must not share** |
| `FixModel` trait / shared engine | **Rejected** |

Naming `src/casing/` (not `engine/`) is intentional.

---

## `casing/` layout

```
casing/
├── fixer.rs
├── config.rs           # language + models_dir
└── backend/            # private
    ├── mod.rs          # raw ASR restore vs normalize
    ├── tokens.rs
    ├── restore.rs      # uses installed Lexicon
    └── normalize.rs
```

Do **not** expose rule engine / ONNX / Candle in `cli.md`.

---

## `artifact/` layout

Not a parser of meaning. Knows only how to find **mutable transcript text spans**.

```
artifact/
├── detect.rs
├── load.rs
├── writer.rs
├── text_spans.rs       # apply_to_text_spans + count_text_spans
└── formats/
```

---

## Public fix API

```rust
let fixer = CasingFixer::load(CasingLoadOptions {
    language: Language::Ru,
    models_dir: paths::resolve_models_dir(None),
})?;  // Err only on real backend init failure (e.g. corrupt pack) → exit 4

apply_to_text_spans(&mut artifact, |span| {
    let result: FixResult = fixer.fix(span.text, FixOptions::default())?;
    if result.changed {
        *span.text = result.text;
    }
    Ok(())
})?;

write(&artifact, &output_path)?;
```

---

## Guarantees in code

| Layer | Responsibility | Enforces |
|-------|----------------|----------|
| `models/` | packs | `run` cannot fix without installed language pack |
| `artifact/` | structure | Only `TextSpan` is mutable; type preserved on write |
| `casing/` | text | Presentation only — no ASR repair, terms, translate, sentence rewrite |
| `output/` | filesystem | `.fixed.{ext}`; `-o` XOR `-d` XOR `--in-place` |
| `cli/` | UX | Exit codes from [cli.md](cli.md); dry-run never writes |

---

## Progress

| Command | Events |
|---------|--------|
| `install` | `start` → `downloading` (`percent`, bytes) → `done` / `error` |
| `run` | `start` → `loading` → `processing` (`percent`, `span`/`span_total`) → `writing` → `done` / `error` |

Same `--progress=text\|json` and `-q` as [cli.md](cli.md) / `vd-gigaam`.

---

## Tests and fixtures

All tests under `tests/` — **no** `#[cfg(test)]` in `src/`.

| Path | Role |
|------|------|
| `tests/unit/cli.rs` | clap / conflicts |
| `tests/unit/output_paths.rs` | `-o` / `-d` / `--in-place` / `.fixed.` |
| `tests/unit/artifact_*.rs` | detect / spans / roundtrip |
| `tests/unit/casing.rs` | builtin without pack; rewrite; word identity |
| `tests/e2e/binary.rs` | install optional, progress %, dry-run pack status |

```bash
cargo test -p vd-fix-casing --test unit
cargo test -p vd-fix-casing --test e2e
```

---

## Build

```bash
cd cli/fix/vd-fix-casing
cargo build --release
cargo test
cargo run -- install ru
cargo run -- run -i fixtures/input/sample.txt --dry-run
cargo run -- run -i fixtures/input/sample.txt --progress=json
```

Binary name: `vd-fix-casing`.  
Workspace member: `cli/fix/vd-fix-casing` (root `Cargo.toml`).
