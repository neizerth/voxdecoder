# vd-fix-casing — project layout

Rust crate for the presentation-fix CLI.

Related: [README.md](README.md) (product notes) · [cli.md](cli.md) (flags) · [RUST.md](RUST.md) (fmt / clippy) · [TODO-languages.md](TODO-languages.md)

---

## Philosophy

**Backend is an implementation detail.**

This is a project rule for every `vd-fix-*` binary, not only casing. Tomorrow the backend may be Candle, ONNX Runtime, llama.cpp, regex, a rule engine, or an ensemble — **none of that leaks** into `cli.md`, public modules, progress events, or dry-run JSON.

Exit code 4 stays backend-agnostic: *Inference backend failed to initialize.*

Same idea for naming: `casing/` / future `asr/` / `terms/` — job names, never `engine/` / `model/` / `pipeline/`.

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
│   │   └── config_cmd.rs
│   ├── config/                 # load / save / merge / defaults
│   │   ├── mod.rs
│   │   ├── file.rs
│   │   └── resolve.rs          # CLI > config > default → ResolvedConfig
│   ├── artifact/               # mutable text-span iterator over artifacts
│   │   ├── mod.rs
│   │   ├── detect.rs           # extension / sniff → ArtifactType
│   │   ├── load.rs
│   │   ├── writer.rs           # ArtifactWriter — serialize same type
│   │   ├── text_spans.rs       # → TextSpan (only mutable transcript text)
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
│   ├── paths.rs                # config path, env overrides
│   └── casing/                 # this binary only — not a shared fix engine
│       ├── mod.rs
│       ├── fixer.rs            # CasingFixer::load / .fix → FixResult
│       ├── config.rs           # load options (language, …)
│       └── backend.rs          # private: init + rewrite presentation
│
├── tests/
│   ├── unit/                   # library API, no process spawn
│   │   ├── mod.rs              # harness (`cargo test --test unit`)
│   │   ├── cli.rs
│   │   ├── output_paths.rs
│   │   ├── artifact_detect.rs
│   │   ├── artifact_text_spans.rs
│   │   ├── artifact_roundtrip.rs
│   │   └── casing.rs
│   └── e2e/                    # spawn `vd-fix-casing` binary
│       ├── mod.rs              # harness (`cargo test --test e2e`)
│       └── binary.rs
│
└── fixtures/                   # committed inputs / expected outputs
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

pub struct ResolvedConfig { /* … */ }
pub struct FixOptions { /* reserved; keep thin */ }

/// Only handle the fixer may mutate. Timestamps / ids / metadata are unreachable.
pub struct TextSpan<'a> {
    pub text: &'a mut String,
}

pub struct FixResult {
    pub text: String,
    pub changed: bool,
    // later, without breaking callers:
    // pub change_count: usize,
    // pub warnings: Vec<String>,
}
```

`TextSpan` is the strongest guarantee in the architecture: `casing` never sees raw artifact JSON/SRT — only spans. Accidentally rewriting a timestamp becomes a type error, not a review comment.

`FixResult` (not bare `String`) leaves room for `change_count` / warnings later without changing the method shape.

---

## Modules

| Path | Role |
|------|------|
| `types.rs` | Domain model: `ArtifactType`, `Language`, `TextSpan`, `FixResult`, … |
| `cli/` | UX: commands from [cli.md](cli.md) (`run`, `config`) |
| `config/` | Persist + merge into `ResolvedConfig` |
| `artifact/` | Find mutable text spans; load/write **same type** |
| `output/` | Filesystem paths only |
| `progress.rs` | stderr progress; stdout free |
| `paths.rs` | Platform config dir |
| `casing/` | Presentation rewriter (text only) |

`paths.rs` and `progress.rs` stay flat — small, stable. `artifact/` and `progress` are the most likely first extracts when `vd-fix-asr` / `vd-fix-terms` appear.

---

## Layer responsibilities

| Layer | Responsibility | Owns |
|-------|----------------|------|
| `artifact/` | **structure** | detect, load, `TextSpan` iteration, `ArtifactWriter` |
| `casing/` | **text** | presentation rewrite inside a span |
| `output/` | **filesystem** | resolve output path (`.fixed.`, `-o`/`-d`/`--in-place`) |
| `cli/` | **UX** | flags, dry-run, progress, exit codes |

Pipeline:

```text
output::path          → where to write
artifact::load        → typed artifact in memory
artifact::text_spans  → TextSpan<'_>
casing::fixer.fix   → FixResult  (apply back onto span)
ArtifactWriter        → same ArtifactType on disk
```

`output/path` never serializes content. `artifact/writer` never decides the path. No double “write” ownership.

---

## Shared crates?

**Default: none.** Keep helpers inside `vd-fix-casing`.

**Duplicate until it hurts.** Three copies are cheaper than one wrong abstraction.

Extract a shared crate only when a second `vd-fix-*` CLI is real **and** the duplicated code is identical (not “almost”).

| Tempting to share | Reality |
|-------------------|---------|
| Artifact detect / text spans / writer | Similar UX across `vd-fix-*`; duplicate until it hurts |
| `--progress` NDJSON | Convention; optional late extract |
| `-i` / `-o` / `-d` / `--in-place` / `.fixed.` | Same family contract; duplicate until it hurts |
| `config` TOML merge | Same pattern; keep local |
| `types` (`TextSpan`, `ArtifactType`, …) | Copy first; extract only if bit-identical |
| Presentation / ASR / terms backends | **Different jobs — must not share** |
| `FixModel` trait / shared engine | **Rejected** |

Even after an extract: helpers only (`artifact`, `progress`, maybe `output::path`), never a multi-fix façade.

Naming `src/casing/` (not `engine/` / `processor/` / `model/` / `pipeline/`) is intentional. Future: `vd-fix-asr` → `src/asr/`, `vd-fix-terms` → `src/terms/`.

---

## `casing/` layout

```
casing/
├── fixer.rs      # public API → FixResult
├── config.rs     # CasingLoadOptions (language, …)
└── backend.rs    # private implementation detail
```

Do **not** expose Candle / ONNX / llama.cpp / regex / rule engine / ensemble in `cli.md` or the public module surface.

If `backend.rs` grows past ~200 lines or sprouts multiple strategies, split under `casing/backend/` (still private).

---

## `artifact/` layout

Not a parser of meaning. Not an ASR / punctuation / terms / LLM layer.

`artifact/` knows only:

> how to find the mutable transcript text spans inside an artifact.

It is a **mutable text-span iterator** (`text_spans`) plus load/write for **input type == output type**.

```
artifact/
├── detect.rs
├── load.rs
├── writer.rs       # ArtifactWriter
├── text_spans.rs   # yields TextSpan<'_>
└── formats/        # one module per ArtifactType
```

`text_spans` must not expose: segment boundaries, timestamps, speaker labels, ids, metadata. Guarantees become type-system facts via `TextSpan`, not only docs.

---

## Public fix API

```rust
let fixer = CasingFixer::load(CasingLoadOptions {
    language: Language::Ru,
})?;

for span in artifact.text_spans() {
    let result: FixResult = fixer.fix(span.text, FixOptions {
        // reserved; keep thin
    })?;
    if result.changed {
        *span.text = result.text;
    }
}

ArtifactWriter::write(&artifact, &output_path)?;
```

Not free functions `load` / `fix` — method style keeps state on `CasingFixer`.

---

## Guarantees in code

| Layer | Responsibility | Enforces |
|-------|----------------|----------|
| `artifact/` | structure | Only `TextSpan` is mutable; type preserved on write |
| `casing/` | text | Presentation only — no ASR repair, terms, translate, sentence rewrite |
| `output/` | filesystem | `.fixed.{ext}`; `-o` XOR `-d` XOR `--in-place` |
| `cli/` | UX | Exit codes from [cli.md](cli.md); dry-run never writes |

Unit tests lock these: structured fixtures with unchanged timestamps/ids; casing fixtures where word identity is preserved; `FixResult.changed == false` on identity input.

---

## Tests and fixtures

All tests live under `tests/` — **no** `#[cfg(test)]` modules in `src/`.

Cargo does not auto-discover subdirs; harnesses are declared in `Cargo.toml` as `[[test]]` → `tests/unit/mod.rs` and `tests/e2e/mod.rs`.

| Path | Role |
|------|------|
| `tests/unit/cli.rs` | clap / exit codes / flag conflicts (`-o`+`-d`, …) |
| `tests/unit/output_paths.rs` | `-o` / `-d` / `--in-place` / `.fixed.` / `--overwrite` |
| `tests/unit/artifact_detect.rs` | extension / sniff → `ArtifactType` |
| `tests/unit/artifact_text_spans.rs` | only `TextSpan`; structure unreachable |
| `tests/unit/artifact_roundtrip.rs` | load → identity spans → write; type preserved |
| `tests/unit/casing.rs` | presentation rewrite; words unchanged; `FixResult` |
| `tests/e2e/binary.rs` | spawn `vd-fix-casing`; exit codes, I/O, progress, dry-run |
| `fixtures/input/` | sample `txt` / `json` / `srt` / … |
| `fixtures/expected/` | expected `.fixed.*` outputs |

```bash
cargo test -p vd-fix-casing --test unit
cargo test -p vd-fix-casing --test e2e
```

Keep fixtures committed next to tests — not generated at runtime.

---

## Build

```bash
cd cli/fix/vd-fix-casing
cargo build --release
cargo test
cargo run -- run -i fixtures/input/sample.txt --dry-run
cargo run -- run -i fixtures/input/sample.txt --progress=json
```

Binary name: `vd-fix-casing`.

Workspace: add this crate to the root `[workspace].members` when scaffolding starts.
