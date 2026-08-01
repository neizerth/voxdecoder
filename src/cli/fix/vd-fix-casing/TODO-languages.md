# TODO — languages beyond `ru` / `en`

`--language` is already in the CLI ([cli.md](cli.md)); default is `ru`. Expanding the set should not change the flag surface — add a catalog pack + `install <code>`.

## Shipping languages

| Code | Status | Notes |
|------|--------|-------|
| `ru` | default / shipping | builtin lexicon; optional `install ru` |
| `en` | shipping | builtin lexicon; optional `install en` |

## Done (near term)

- [x] Accept `ru` and `en` at CLI / config; unknown codes → exit 2.
- [x] Wire `language` into `CasingFixer::load` (installed pack or builtin).
- [x] Pack install UX (`install` / `remove` / `list` / `info`); packs optional for `run`.
- [x] Unit / e2e coverage for install + `ru` / `en` rewrite without install.

## Later

| Code | Status | Notes |
|------|--------|-------|
| `de` | TODO | Catalog reserved (`shipping = false`); noun capitalization; quote style |
| `auto` | TODO | Detect from artifact / heuristic; resolve to installed pack (`ru` fallback) |

Add rows here when a new language is requested — do not invent CLI flags per language.

## Per-language work (repeat for each new code)

- [ ] Catalog entry + embedded (or remote) pack assets.
- [ ] Presentation rules or backend weights for that language.
- [ ] Fixtures under `fixtures/input/` + `fixtures/expected/` tagged by language.
- [ ] Unit tests in `tests/unit/casing.rs` (words unchanged; presentation correct).
- [ ] e2e: `install <code>` then `run -i … --language <code> --dry-run` shows `model` / `installed`.

## Out of scope for language work

- Translation (never — see Guarantees).
- ASR repair / term normalization (other `vd-fix-*` CLIs).
- Changing artifact type or structure.
