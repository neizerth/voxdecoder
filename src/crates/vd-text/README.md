# vd-text

Shared local linguistic primitives for `vd-fix-*` CLIs (ADR 0013).

**Status: Rust-native pieces only.** No CLI, no business logic — a library crate other `vd-fix-*` binaries call into, same shape as `vd-artifact` / `vd-output` / `vd-progress`.

## Owns

- **`term_matcher`** — deterministic multi-pattern terminology matching, `variant -> canonical`, backed by `aho-corasick`. Case-sensitive by default; `TermMatcher::new_ascii_case_insensitive` for ASCII-only case folding.
- **`similarity`** — `edit_distance` / `similarity_ratio` (normalized Levenshtein), backed by `strsim`. One shared implementation instead of `vd-fix-asr`'s `context_fuzzy` and `vd-fix-overlap`'s `overlap::detect` each carrying their own hand-rolled copy (not migrated yet — see below).

## Does not own (yet)

Per ADR 0013: tokenization, sentence segmentation, morphology, and the declarative rule engine all need either a Natasha/razdel subprocess bridge (not built) or a resolved design question (rule-engine duplication with ADR 0010's Stage/Rule model, also not resolved). See [`docs/adr/0013-local-linguistic-infrastructure.md`](../../../docs/adr/0013-local-linguistic-infrastructure.md) for the full picture and what's still open.

## Migrating existing hand-rolled implementations

Not done as part of adding this crate — a natural follow-up, not attempted here to keep this change reviewable on its own:

- `vd-fix-asr/src/asr/context_fuzzy.rs::edit_distance` → `vd_text::similarity::edit_distance`
- `vd-fix-overlap/src/overlap/detect.rs::edit_distance`/`similarity_ratio` → `vd_text::similarity::*`
- `vd-fix-terms`'s per-token `HashMap` lexicon lookup could adopt `term_matcher` for substring/multi-word term matching (its current exact-whole-token lookup doesn't need it, but any future substring-matching requirement would)

```bash
cargo test -p vd-text
```
