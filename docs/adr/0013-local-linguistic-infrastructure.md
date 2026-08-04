# ADR 0013 — Local Linguistic Infrastructure (`vd-text`)

**Status:** Partially implemented — `vd-text` crate shipped with Rust-native pieces (`term_matcher`, `similarity`); Python-dependent pieces (tokenization, sentence segmentation, morphology, rule engine) not started (see Decision)  
**Type:** ADR / architectural RFC  
**Date:** 2026-08-03

**Related:**

- [`vd-fix-asr`](../../src/cli/fix/vd-fix-asr/) · [ADR 0010](0010-vd-fix-asr-local-transcript-cleanup.md)
- [`vd-fix-disfluency`](../../src/cli/fix/vd-fix-disfluency/) · [`vd-fix-overlap`](../../src/cli/fix/vd-fix-overlap/) · [ADR 0012](0012-local-cleanup-disfluency-and-overlap.md)
- [`vd-fix-terms`](../../src/cli/fix/vd-fix-terms/)
- [`vd-fix-layout`](../../src/cli/fix/vd-fix-layout/)
- [`vd-pipeline`](../../src/cli/process/vd-pipeline/)

---

## Motivation

The majority of transcript quality issues do not require LLMs. They originate from ASR artifacts, spontaneous speech, duplicated text, formatting inconsistencies, and terminology normalization — all solvable deterministically using mature NLP libraries plus VoxDecoder-specific rules.

Rather than embedding NLP logic inside every `vd-fix-*` binary, this ADR proposes a shared linguistic infrastructure crate.

## Goals

- Maximize transcript quality without LLMs.
- Keep every `vd-fix-*` focused on a single responsibility.
- Reuse NLP components across capabilities.
- Make language-specific rules configurable.
- Support future language packs.

---

## Resolved: Natasha/razdel path

**`Natasha` and `razdel` are Python packages, not Rust crates.** This entire repository is a Rust Cargo workspace; there is no existing Python interop layer anywhere in this codebase today. Three paths were considered (subprocess/FFI bridge, Rust-native substitutes, narrowing scope to Rust-only pieces) — **the subprocess bridge is chosen.**

### Why subprocess, not `pyo3`

`pyo3` embeds a Python interpreter *inside* the Rust process (same address space, GIL, shared crash domain — a segfault or unhandled Python exception in Natasha can take the whole `vd-fix-*` binary down with it). A subprocess bridge keeps the exact same isolation model this repo already uses everywhere else: `vd-pipeline` and every `vd-fix-*`/`vd-meeting` capability already talks to *other* `vd-*` binaries as child processes over argv + files (see `vd-pipeline/src/exec/subprocess.rs`). A Python sidecar is architecturally the same shape — one more kind of child process — rather than a new, heavier-weight integration model. It's also far easier to make optional: a `vd-fix-*` binary can detect the Python sidecar is missing and fall back to its existing pure-Rust rules (already true for `vd-fix-asr` — see below), whereas an embedded interpreter is compiled in either way.

### Design sketch

```text
vd-text (Rust)                      vd-text-py (Python sidecar, new)
  ├─ spawns / talks to  ──────────►    natasha + razdel wrapped in a
  │  a long-lived subprocess           thin stdin/stdout JSON-lines
  │  over stdin/stdout (NDJSON          protocol: {"op": "tokenize",
  │  request/response, one line          "text": "…"} → {"tokens": […]}
  │  per call — not spawn-per-call,
  │  process start-up dominates
  │  Natasha's own model-load cost)
  └─ falls back to existing pure-
     Rust rules if the sidecar is
     unavailable (missing Python,
     missing packages, subprocess
     spawn failure) — never a hard
     dependency for the default,
     fully-local `vd-fix-asr` path
```

- **New component**: `src/python/vd-text-py/` (Python sidecar next to Rust under `src/`; not a Cargo crate) — a small Python package pinning `natasha` + `razdel`, exposing tokenize/morphology/sentence-segmentation over stdin/stdout NDJSON. Long-lived process (spawned once, reused across calls) to amortize Natasha's model-load time, not spawned per text span.
- **Packaging cost, stated plainly**: this is the real departure ADR 0010 bet against — `vd-fix-asr`'s "fully local, deterministic, single static binary" claim no longer holds for whichever features route through `vd-text-py`. Deploys that want Natasha-backed morphology need a Python 3.x runtime + `pip install natasha razdel` reachable at runtime (venv, system Python, or a bundled interpreter — packaging mechanism is a separate decision, not resolved here).
- **Never a hard requirement for the default path**: `vd-fix-asr`'s Stage 1–6 pipeline (ADR 0010, already shipped) stays exactly as it is — no stage currently depends on `vd-text`. Any future stage that *wants* Natasha morphology must degrade gracefully (skip that specific enhancement, not fail the whole run) when the sidecar isn't present. This preserves ADR 0010's guarantee for everyone who never installs the Python sidecar.
- **Aho-Corasick / RapidFuzz-equivalent pieces are unaffected** by any of this — native Rust crates (`aho-corasick`, `strsim`/`rapidfuzz-rs`), no subprocess, no packaging cost, can land independently and first.

---

## New shared crate

```text
src/crates/vd-text/
```

`vd-text` provides reusable linguistic primitives. It contains no business logic, owns no CLI, and is shared by every text-processing capability.

### Responsibilities

**Owns:** tokenization, sentence segmentation, morphology adapters, similarity utilities, terminology matching, rule engine, normalization helpers, TimeMap helpers.

**Never owns:** transcript cleanup, layout decisions, terminology policy, diarization, meeting logic.

### Architecture

```text
                vd-text
         ┌────────┼────────┐
         │        │        │
   Linguistics  Rules   Similarity
         │        │        │
         └────────┼────────┘
                   │
       vd-fix-* capabilities
```

---

## External components

| Component | Responsibilities | Used by | Rust availability |
|---|---|---|---|
| **Natasha** | tokenization, morphology, POS, named entities, syntax | `vd-fix-asr`, `vd-fix-disfluency`, `vd-fix-layout` | Python only — see open question |
| **razdel** | sentence + token boundaries | `vd-fix-layout`, `vd-fix-asr` | Python only — see open question |
| **Aho-Corasick** | canonical names, technical vocabulary, company names, APIs, frameworks | `vd-fix-terms`, `vd-fix-asr` | Native Rust crate (`aho-corasick`) — no blocker |
| **RapidFuzz / Levenshtein** | duplicated speech, overlap detection, repeated fragments, fuzzy comparisons | `vd-fix-overlap` | Rust equivalents exist (`rapidfuzz-rs`, `strsim`) — no blocker |

---

## Rule Engine

`vd-text` introduces a declarative cleanup engine — rules are data (YAML), not Rust code:

```yaml
- id: duplicate-word
  when:
    repeated_word: true
  action:
    remove_second
```

```yaml
- id: filler
  when:
    token: эээ
  action:
    remove
```

```yaml
- id: merged-word
  when:
    merged_word: true
  action:
    split
```

Rules may be language-specific.

Note: this is a materially different design from ADR 0010's Stage/Rule model in `vd-fix-asr` (`asr/rule.rs`, `asr/stages/`), which is Rust-code rules composed into fixed-order stages, not a YAML-driven declarative engine. Adopting this ADR would mean either migrating `vd-fix-asr`'s existing stages onto the new engine, or running two rule-engine designs side by side. This reconciliation is not addressed by the source document and needs a decision during design, not silently during implementation.

## Language packs

```text
builtin → ru → en → future languages
```

Each language contributes: filler lists, merge rules, split rules, punctuation rules, discourse markers.

## Shared utilities

Tokenizer (unified token API) · Sentence splitter (razdel-based) · Morphology (Natasha adapter) · Similarity (RapidFuzz wrapper) · Term matcher (Aho-Corasick wrapper) · Rule Engine · TimeMap helpers.

---

## Capability integration

| Capability | Purpose | Uses | Never |
|---|---|---|---|
| `vd-fix-asr` | Repair deterministic ASR artifacts (duplicated/merged/split words, punctuation, mixed alphabets, spelling mistakes) | Natasha, razdel, Rule Engine, Aho-Corasick | Rewrites meaning |
| `vd-fix-disfluency` | Remove speech disfluencies (fillers, accidental repetitions, false starts, hesitations) | Natasha, Rule Engine | Removes meaningful discourse markers |
| `vd-fix-terms` | Normalize terminology, layered dictionaries (builtin → language → project → user) | Aho-Corasick, glossary lookup | Guesses |
| `vd-fix-layout` | Produce readable paragraphs from sentence boundaries, pause duration, discourse markers, paragraph density | Natasha, razdel, TimeMap | Anything beyond layout |
| `vd-fix-overlap` | Remove duplicated speech from diarization overlap (lexical similarity, overlapping timestamps, repeated spans) | RapidFuzz, TimeMap | Removes unique speech |

`vd-fix-disfluency` modes: `off | light | normal | aggressive` — same shape as ADR 0012.

## Pipeline

```text
ASR → vd-fix-asr → vd-fix-disfluency → vd-fix-layout → vd-fix-terms → vd-fix-overlap (meeting only) → postprocess
```

Meeting pipelines insert `vd-fix-overlap` after diarization; single-speaker pipelines skip it automatically. Consistent with ADR 0012's pipeline placement and ADR 0010's fix-asr ordering.

## Future extensions

Number/date/unit normalization, language detection, spelling backends, finite-state rewrite rules, confidence-aware cleanup — none require changing existing `vd-fix-*` APIs, per the source proposal.

## Success criteria

- Introduce shared `src/crates/vd-text`.
- Centralize linguistic infrastructure.
- Reuse Natasha across all language-aware capabilities.
- Use razdel for sentence segmentation.
- Use Aho-Corasick for deterministic terminology matching.
- Use RapidFuzz for overlap detection.
- Introduce a declarative Rule Engine shared across cleanup capabilities.
- Keep every `vd-fix-*` focused on one responsibility while sharing common linguistic components.

---

## Decision

**Partially implemented.**

1. **`src/crates/vd-text` shipped — Rust-native pieces only.** `term_matcher` (Aho-Corasick-backed `variant -> canonical` matching, case-sensitive by default plus an ASCII-case-insensitive constructor) and `similarity` (`edit_distance`/`similarity_ratio`, a thin `strsim` wrapper). No CLI, no business logic — same shape as `vd-artifact`/`vd-output`/`vd-progress`. 15 unit tests, clippy clean. Deliberately **not** migrated into any existing `vd-fix-*` crate yet (`vd-fix-asr::context_fuzzy` and `vd-fix-overlap::overlap::detect` still carry their own hand-rolled `edit_distance` — see `vd-text/README.md` "Migrating existing hand-rolled implementations") — adding the shared crate and migrating callers are kept as separate, independently-reviewable changes.
2. **Natasha/razdel path: resolved, not built.** Subprocess bridge (see "Resolved: Natasha/razdel path" above) — a Python sidecar process (`vd-text-py`, NDJSON over stdin/stdout, long-lived not spawn-per-call), never a hard dependency for `vd-fix-asr`'s already-shipped, fully-local ADR 0010 pipeline. `pyo3` (in-process embedding) was considered and rejected in favor of matching this repo's existing child-process isolation model. Packaging direction for when this is built: system Python + a `venv` `vd-text` provisions on first use (not a bundled/vendored interpreter) — chosen but not implemented. No `vd-text-py` code, no tokenization/sentence-segmentation/morphology in `vd-text` yet.
3. **Still open — rule-engine duplication**: whether this ADR's declarative YAML rule engine replaces or coexists with ADR 0010's Rust-code Stage/Rule model already implemented and shipping in `vd-fix-asr`. Not resolved; needs its own answer before `vd-text` grows a Rule Engine component.

`vd-fix-overlap`'s own similarity code (ADR 0012, already shipped) and `vd-fix-asr`'s dictionary-stage fuzzy matching are the natural first candidates to migrate onto `vd_text::similarity` — deferred, not forgotten.
