# TODO — languages beyond `ru` / `en`

`--language` is already in the CLI ([cli.md](cli.md)); default is `ru`. Expanding the set should not change the flag surface.

## Shipping languages

| Code | Status | Notes |
|------|--------|-------|
| `ru` | default / required | First shipping language |
| `en` | required | Capitalization + quotes/dashes norms differ from RU — ship with `ru` |

## Near term

- [ ] Accept `ru` and `en` at CLI / config; reject unknown codes with exit 2.
- [ ] Wire `language` into `CasingFixer::load` / backend init for both codes.
- [ ] Fixtures + unit/e2e coverage for `ru` and `en`.

## Later

| Code | Status | Notes |
|------|--------|-------|
| `de` | TODO | Noun capitalization; quote style |
| `auto` | TODO | Detect from artifact / heuristic; fall back to `ru` if unsure |

Add rows here when a new language is requested — do not invent CLI flags per language.

## Per-language work (repeat for each code)

- [ ] Presentation rules or backend weights for that language.
- [ ] Fixtures under `fixtures/input/` + `fixtures/expected/` tagged by language.
- [ ] Unit tests in `tests/unit/casing.rs` (words unchanged; presentation correct).
- [ ] e2e: `run -i … --language <code> --dry-run` shows `language` in the plan.

## Out of scope for language work

- Translation (never — see Guarantees).
- ASR repair / term normalization (other `vd-fix-*` CLIs).
- Changing artifact type or structure.
