# vd-fix-terms — project layout

Rust crate for the terminology-fix CLI.

**Status: implemented.** Workspace member `vd-fix-terms`.

Related: [README.md](README.md) (product notes) · [cli.md](cli.md) (flags) · [RUST.md](RUST.md) (fmt / clippy) · [TODO-languages.md](TODO-languages.md) · shared I/O: [`crates/`](../../../crates/)

---

## Philosophy

**Backend is an implementation detail.**

Same project rule as every `vd-fix-*` binary. Tomorrow the matcher may be rules, a trie, an Aho–Corasick table, Candle, or an ensemble — **none of that leaks** into `cli.md`, public modules, progress events, or dry-run JSON.

Exit code 4 stays backend-agnostic: *Backend failed to initialize* (e.g. corrupt pack later). Missing `--terms` is **not** exit 4 when a shipping lexicon exists (unless the caller disabled it).

Naming: `terms/` — job name, never `engine/` / `model/` / `pipeline/`.

**The lexicon is the authority.** Unlike `vd-fix-asr`, this CLI’s source of truth is explicit term sources — not open-ended language-model guessing.

**Optional downloadable packs** (`install` / `remove` / `list` / `info`) are a *possible* future only if needed — same convention as `vd-gigaam` / `vd-fix-casing`. Prefer shipping lexicon + `--terms` first. Do **not** invent an `assets/` module until packs are real.

**Language priority:** `ru` = Russian with English insertions first. See [TODO-languages.md](TODO-languages.md).

---

## Non-goals

`vd-fix-terms` intentionally does **not**:

- invent canonical forms without a dictionary / rule entry
- repair open-ended ASR mishearings (`vd-fix-asr`)
- restyle presentation (`vd-fix-casing`)
- translate
- summarize / rewrite for style
- use audio / re-run ASR
- share a multi-fix engine with casing / asr

---

## Tree (planned)

Crate lives at `src/cli/fix/vd-fix-terms/` in the repo:

```
src/cli/fix/vd-fix-terms/
├── Cargo.toml
├── README.md
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
│   ├── lexicon/                # authority — read-oriented
│   │   ├── mod.rs              # pub struct Lexicon
│   │   ├── shipping.rs         # shipping common tech terminology
│   │   ├── merge.rs            # precedence / last-wins merge
│   │   └── loaders/
│   │       ├── mod.rs
│   │       ├── yaml.rs
│   │       ├── json.rs
│   │       ├── markdown.rs
│   │       └── dir.rs
│   ├── paths.rs                # VD_FIX_TERMS_* via vd_artifact::paths
│   └── terms/                  # this binary only — not a shared fix engine
│       ├── mod.rs
│       ├── fixer.rs            # TermsFixer::new(lexicon) / .fix → FixResult
│       ├── config.rs           # TermsLoadOptions
│       └── backend/            # private matcher / rewrite
│
├── tests/
│   ├── unit/
│   └── e2e/
│
└── fixtures/
    ├── input/
    ├── terms/                  # sample glossaries
    └── expected/
```

**Future expansion:** optional downloadable packs may introduce an `assets/` module (catalog / pack / cache). Do not create that folder until packs ship.

Docs for this binary: `src/cli/fix/vd-fix-terms/` (this folder).

---

## Domain model

Shared types live in **`vd-artifact`** (plus `ProgressFormat` from `vd-progress`). This crate re-exports them via `types.rs`.

```rust
// from vd-artifact + vd-progress (re-exported)
pub enum ArtifactType { /* txt, json, jsonl, srt, vtt, md, … */ }
pub enum Language { Ru, En, De, Auto /* … */ }
pub enum ProgressFormat { Text, Json }

pub struct TextSpan<'a> {
    pub id: SpanId,
    pub index: usize,
    pub text: &'a mut String,
}

pub struct FixResult {
    pub text: String,
    pub changed: bool,
}
```

`FixOptions` is **not** required on the public fix path until a real per-call dial appears (e.g. strict / conservative). Prefer `fixer.fix(text)?` until then.

### `Lexicon` (authority — in `lexicon/`)

```rust
/// Merged variant → canonical map. Read-only after load.
pub struct Lexicon { /* … */ }

impl Lexicon {
    pub fn load(opts: &TermsLoadOptions) -> Result<Self, LexiconError> { /* … */ }
}
```

### `TermsLoadOptions` (load-time — in `terms/config.rs`)

```rust
pub struct TermsLoadOptions {
    pub language: Language,
    /// Include the shipping lexicon (default: true).
    /// Set false for corporate-only glossaries via `--terms`.
    pub shipping: bool,
    /// Paths from repeatable `--terms` (left → right; **last wins** on conflict).
    pub terms_paths: Vec<PathBuf>,
}
```

Unlike `vd-fix-casing`, the fixer may **change words**. Unlike `vd-fix-asr`, changes are **lexicon-authorized** only.

---

## Modules

| Path | Role |
|------|------|
| `vd-artifact` / `vd-output` / `vd-progress` | see [../../../crates/README.md](../../../crates/README.md) |
| `types.rs` | Re-export shared crate types |
| `cli/` | UX from [cli.md](cli.md) |
| `config/` | Persist + merge into `ResolvedRun` |
| `lexicon/` | **Authority** — `Lexicon` + shipping + loaders + merge |
| `paths.rs` | `VD_FIX_TERMS_*` via `vd_artifact::paths` |
| `terms/` | Apply canonical forms (text + loaded `Lexicon`) |

---

## Lexicon (`lexicon/`) — authority

```text
shipping lexicon?   +   --terms paths (ordered)
        ↓ merge (see Source precedence)
     Lexicon
```

Loaders stay small and format-specific under `lexicon/loaders/`. Merge owns precedence — not the individual loaders.

On-disk glossary shape is backend-private; product examples live in [README.md](README.md#glossary-shape-illustrative).

---

## Source precedence

Highest priority first:

1. **`--terms` (CLI)** — each path in left-to-right order; **last wins** on the same variant
2. **user config** (optional future: default terms path list; same last-wins within that list)
3. **shipping lexicon** (if `TermsLoadOptions.shipping == true`)

Examples:

```text
--terms a.yaml --terms b.yaml
→ b.yaml overrides a.yaml on shared variants
→ both override shipping
```

```text
--terms corp.yaml  +  shipping: false
→ only corp.yaml; no shipping entries
```

A lower source never invents a replacement that a higher source already defined differently.

Product summary: [README.md](README.md#sources-precedence). CLI flag for disabling shipping: planned (e.g. `--no-shipping-lexicon`) — see [cli.md](cli.md).

---

## Layer responsibilities

| Layer | Responsibility | Owns / authority |
|-------|----------------|------------------|
| `lexicon/` | **authority** | variant → canonical maps (`Lexicon`) |
| `vd-artifact` | **structure** | detect, load, span walk, write |
| `terms/` | **text** | apply canonical forms inside the current span |
| `vd-output` | **filesystem** | `.fixed.`, `-o`/`-d`/`--in-place` |
| `cli/` | **UX** | flags, dry-run, progress, exit codes |

Authority across `vd-fix-*` (product model):

| CLI | Authority |
|-----|-----------|
| `vd-fix-casing` | typography / presentation rules |
| `vd-fix-asr` | transcript meaning (+ local context) |
| `vd-fix-terms` | lexicon |

### Pipeline (planned)

```text
artifact::load                 → typed artifact
count / index text spans       → progress denominator
Lexicon::load                  → shipping? + --terms (read-only authority)
output::path                   → where to write (fixed_file_name)
apply_to_text_spans            → TextSpan<'_>
  TermsFixer::fix             → FixResult (words may change; lexicon only)
artifact::write                → same ArtifactType
```

Prepare authority before writing paths; both happen before the fix loop.

---

## Shared crates?

**Yes — lean on [`crates/`](../../../crates/).** Formats/spans → `vd-artifact`; `.fixed.` → `vd-output`; progress → `vd-progress`. Do not copy those modules back into the CLI crate.

| Keep in shared crates | Keep in this binary |
|------------------------|---------------------|
| Artifact / progress / `.fixed.` / `paths` helpers | `lexicon/` + `terms/` backend |
| `ArtifactType`, `TextSpan { id, index, text }`, … | pack install UX **only when packs exist** |

| Rejected |
|----------|
| Shared presentation / ASR / terms engine |
| `FixModel` trait |

Naming `src/terms/` (not `engine/`) is intentional.

---

## Public fix API (planned)

```rust
let lexicon = Lexicon::load(&TermsLoadOptions {
    language: Language::Ru,
    shipping: true,
    terms_paths,
})?;  // Err only on real lexicon init failure → exit 4

let fixer = TermsFixer::new(lexicon)?;

apply_to_text_spans(&mut artifact, |span| {
    let result: FixResult = fixer.fix(span.text)?;
    if result.changed {
        *span.text = result.text;
    }
    Ok(())
})?;

write(&artifact, &output_path)?;
```

When a per-call dial appears later, add an overload (e.g. `fix_with(text, opts)`) — do not force empty `FixOptions` on every call site now.

Dry-run / CLI surface stays as in [cli.md](cli.md).

---

## Guarantees in code

| Layer | Responsibility | Enforces |
|-------|----------------|----------|
| `lexicon/` | authority | no rewrite without a map entry |
| `vd-artifact` | structure | Only `TextSpan::text` is mutable; type preserved on write |
| `terms/` | text | Canonical terminology only — see Non-goals |
| `vd-output` | filesystem | `.fixed.{ext}`; `-o` XOR `-d` XOR `--in-place` |
| `cli/` | UX | Exit codes from [cli.md](cli.md); dry-run never writes |

---

## Progress

| Command | Events |
|---------|--------|
| `install` (future) | `start` → `phase downloading` → `done` / `error` |
| `run` | `start` → `phase loading` → `phase processing` (`percent`, `span`/`span_total`) → `phase writing` → `done` / `error` |

Same `--progress=text|json` and `-q` as [cli.md](cli.md) / [`vd-progress`](../../../crates/vd-progress/).

---

## Tests and fixtures (planned)

All tests under `tests/` — **no** `#[cfg(test)]` in `src/`.

| Path | Role |
|------|------|
| `tests/unit/cli.rs` | clap / conflicts / `--no-shipping-lexicon` |
| `tests/unit/lexicon.rs` | loaders, merge, last-wins, shipping on/off |
| `tests/unit/terms.rs` | canonicalization; no invent; structure untouched |
| `tests/e2e/binary.rs` | progress spans, dry-run, `--terms` |
| `src/crates/*/tests/unit/` | artifact / output paths |

```bash
# after crate exists:
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-fix-terms --test unit
cargo test -p vd-fix-terms --test e2e
```

---

## Build (planned)

```bash
cd src/cli/fix/vd-fix-terms
cargo build --release
cargo test
cargo run -- run -i fixtures/input/sample.txt --dry-run
```

Binary name: `vd-fix-terms`.  
Workspace member: `src/cli/fix/vd-fix-terms` (add to root `Cargo.toml` when implementing; depends on `vd-artifact` / `vd-output` / `vd-progress`).

---

## Public contract note

Dictionary format, matcher, and backend implementation are intentionally **outside** the public CLI contract.
