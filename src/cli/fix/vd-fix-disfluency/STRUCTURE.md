# vd-fix-disfluency — project layout

Rust crate for the speech-disfluency cleanup CLI.

**Status: implemented.** Workspace member: `src/cli/fix/vd-fix-disfluency`.

Related: [README.md](README.md) (product notes) · [cli.md](cli.md) (flags) · [RUST.md](RUST.md) (fmt / clippy) · [ADR 0012](../../../docs/adr/0012-local-cleanup-disfluency-and-overlap.md) · shared I/O: [`crates/`](../../../crates/)

---

## Philosophy

**Deterministic, table-driven, no LLM.** Same local-first rule as every `vd-fix-*` binary (ADR 0010 / ADR 0012). No regex dependency: a small hand-rolled word/separator tokenizer (`disfluency::rules`), same spirit as `vd-fix-asr`'s `backend::next_token`.

**No context needed.** Unlike `vd-fix-asr`, disfluency detection does not need neighboring segments or `--context` materials — every rule operates on a single span's text in isolation. There is intentionally no `context/` module here.

**Language priority:** `ru` first, mirrors `vd-fix-asr`'s `Language::En => En, _ => Ru`.

---

## Non-goals

`vd-fix-disfluency` intentionally does **not**:

- fix misrecognized words / homophones (`vd-fix-asr`)
- restyle presentation, casing, or whitespace as a job (`vd-fix-casing`)
- reflow paragraphs / segment layout (`vd-fix-layout`)
- normalize terminology to project-canonical forms (`vd-fix-terms`)
- remove duplicated speech across speakers (`vd-fix-overlap`)
- translate
- use audio / re-run ASR
- infer missing transcript content
- change segment boundaries, timestamps, speaker labels, ids, or metadata

### Known scaffold limitations (not covered by tests, documented on purpose)

- **False-start detection is exact-repeat only.** `Я... я думаю` is caught (same word, case-insensitive); a truncated restart like `Дума... думаю` (stem prefix, not exact repeat) is not. Widening this to a stem/prefix heuristic was deliberately deferred — it increases false-positive risk (unrelated short words sharing a prefix) and the ADR's own example is an exact repeat.
- **`aggressive` == `normal`.** No additional rule set exists yet for `aggressive`; it is reserved for future riskier transforms once a concrete need appears.
- **Empty-hesitation cleanup handles a single filler between two words** (`word... filler... word` → `word, word`). Chains of 2+ consecutive hesitation groups in a row may leave a minor punctuation artifact (e.g. a stray `,...`) instead of one clean separator — not exercised by the test suite; a real-world `--mode normal` run still removes the filler content, just not always with picture-perfect punctuation in this compound case.
- **Protected-phrase guard covers the false-start rule only** (the riskiest transform). It is not needed for filler removal: filler tokens (`эээ`, `ммм`, `эм`, `um`, `uh`, `erm`) are whole-word matched via the tokenizer and none of them collide with any protected phrase.

---

## Tree

```
src/cli/fix/vd-fix-disfluency/
├── Cargo.toml
├── README.md                   # product notes
├── cli.md                      # flags
├── STRUCTURE.md                # this file
├── RUST.md                     # fmt / clippy
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── types.rs                # re-export shared crates + this crate's Mode
│   ├── paths.rs                # VD_FIX_DISFLUENCY_* via vd_artifact::paths
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── run.rs
│   │   └── config_cmd.rs
│   ├── config/
│   │   ├── mod.rs
│   │   ├── file.rs
│   │   └── resolve.rs
│   └── disfluency/              # this binary only — not a shared fix engine
│       ├── mod.rs
│       ├── fixer.rs            # DisfluencyFixer::load / .fix → FixResult
│       └── rules.rs            # filler tables, protected phrases, rule pipeline
│
├── tests/
│   ├── unit/
│   │   ├── mod.rs
│   │   ├── cli.rs
│   │   └── disfluency.rs
│   └── e2e/
│       ├── mod.rs
│       └── binary.rs
│
└── fixtures/
    ├── input/sample.txt
    └── expected/sample.fixed.txt
```

---

## Domain model

Shared types live in **`vd-artifact`** (plus `ProgressFormat` from `vd-progress`). This crate re-exports them via `types.rs`, plus its own `Mode` (`disfluency::rules::Mode`).

```rust
pub enum ArtifactType { /* txt, json, jsonl, srt, vtt, md, … */ }
pub enum Language { Ru, En, De, Auto }
pub enum ProgressFormat { Text, Json }

/// Only handle the fixer may mutate. Timestamps / ids / metadata are unreachable.
pub struct TextSpan<'a> {
    pub id: SpanId,
    pub index: usize,
    pub text: &'a mut String,
}

pub struct FixResult {
    pub text: String,
    pub changed: bool,
}

/// off | light | normal | aggressive — this crate's own type (not in vd-artifact,
/// since it is specific to disfluency cleanup strength).
pub enum Mode { Off, Light, Normal, Aggressive }
```

### `DisfluencyLoadOptions` (in `disfluency/fixer.rs`)

```rust
pub struct DisfluencyLoadOptions {
    pub language: Language,
    /// Effective mode — `config::resolve_run` already folds `remove_fillers = false`
    /// into `Mode::Off` before this is constructed.
    pub mode: Mode,
}
```

Unlike `vd-fix-asr`, the fixer cannot fail to load (no dictionary, no model, no I/O) — `DisfluencyFixer::load` is `Ok`-only today. It stays `Result`-returning to match the sibling `vd-fix-*` shape and to avoid a signature break if a future rule source (e.g. `--context`-supplied filler lists) needs one.

---

## Modules

| Path | Role |
|------|------|
| `vd-artifact` / `vd-output` / `vd-progress` | see [../../../crates/README.md](../../../crates/README.md) |
| `types.rs` | Re-export shared crate types + `Mode` |
| `cli/` | UX from [cli.md](cli.md) |
| `config/` | Persist + merge into `ResolvedRun` (`language`, `mode`, `remove_fillers`, `in_place`, `progress`) |
| `paths.rs` | `VD_FIX_DISFLUENCY_*` via `vd_artifact::paths` |
| `disfluency/` | Tokenizer + rule pipeline + public `DisfluencyFixer` API |

---

## Layer responsibilities

| Layer | Responsibility | Owns |
|-------|----------------|------|
| shared crates | **structure + paths** | `vd-artifact` + `vd-output` |
| `disfluency/` | **text** | filler / hesitation / false-start rules inside the current span only |
| `cli/` | **UX** | flags, dry-run, progress, exit codes |

### Pipeline

```text
artifact::load               → typed artifact (know ArtifactType)
count text spans              → progress denominator
output::path                  → where to write (may depend on type)
apply_to_text_spans            → for each span
  disfluency::fixer.fix        → FixResult (text may shrink; this span only)
artifact::write                → same ArtifactType
```

Output path is resolved **after** load so extension / type-aware naming stays honest.

---

## Rule pipeline (`disfluency/rules.rs`)

Pure, allocation-light text transform, no I/O:

```text
tokenize                      → Vec<Chunk> (Word | Sep), lossless — chunks re-concat to the original text
apply_empty_hesitation        → word … filler … word  ⇒  word, word     (light+)
collapse_fillers               → isolated fillers removed; runs collapsed (light) or removed (normal+)
strip_trailing_backchannels    → … substance. Угу. Угу. ⇒ … substance. ; sole Угу. kept (light+)
collapse_echo_repeats          → Ну давай. Давай, давай. ⇒ Ну давай.   (allowlisted; light+)
merge_seps + normalize_all_seps → clean up doubled punctuation left behind
[normal+] collapse_false_starts → word … word continuation  ⇒  word continuation, guarded by protected phrases
[normal+] merge_seps + normalize_all_seps  → clean up again
render                         → String
apply_glued_onset_pass         → Ччисто ⇒ Чисто                          (via vd-text, post-render)
```

`Mode::Off` short-circuits before tokenizing — a true no-op, `changed = false`.

---

## Guarantees in code

| Layer | Responsibility | Enforces |
|-------|----------------|----------|
| `vd-artifact` | structure | Only `TextSpan::text` is mutable; type preserved on write |
| `disfluency/` | text | Speech-noise removal only — see Non-goals; protected phrases never touched |
| `vd-output` | filesystem | `.fixed.{{ext}}`; `-o` XOR `-d` XOR `--in-place` |
| `cli/` | UX | Exit codes from [cli.md](cli.md); dry-run never writes |

---

## Progress

| Command | Events |
|---------|--------|
| `run` | `start` → `loading` → `processing` (`percent`, `span`, `span_total`) → `writing` → `done` / `error` |

Same `--progress=text\|json` and `-q` as [cli.md](cli.md).

---

## Tests and fixtures

All tests under `tests/` — **no** `#[cfg(test)]` in `src/`.

| Path | Role |
|------|------|
| `tests/unit/cli.rs` | clap / conflicts / mode & no-fillers flags |
| `tests/unit/disfluency.rs` | mode gating (off/light/normal/aggressive), filler removal per language, repeated-run collapse, empty hesitation, false starts, protected-phrase guard |
| `tests/e2e/binary.rs` | end-to-end run, dry-run (text + json), `--no-fillers`, progress span/percent, exit codes, config roundtrip |
| `src/crates/*/tests/unit/` | artifact detect / spans / roundtrip / output paths |

```bash
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-fix-disfluency --test unit
cargo test -p vd-fix-disfluency --test e2e
```

---

## Build

```bash
cd src/cli/fix/vd-fix-disfluency
cargo build --release
cargo test
cargo run -- run -i fixtures/input/sample.txt --dry-run
```

Binary name: `vd-fix-disfluency`.
Workspace member: `src/cli/fix/vd-fix-disfluency` (depends on `vd-artifact` / `vd-output` / `vd-progress`).

---

## `vd-pipeline` wiring

`Capability::FixDisfluency` is in `vd-pipeline`'s `job/default.rs::default_job()`, between `fix-asr` and `fix-terms` (matching ADR 0012's pipeline order), dispatched via the same generic `run_fix(req, "vd-fix-disfluency")` helper every other `vd-fix-*` capability uses — no special-casing needed since this crate's CLI shape (`-i`/`-o`/`-d`/`--in-place`/`--overwrite`/`-l`/`-q`) matches the convention exactly.
