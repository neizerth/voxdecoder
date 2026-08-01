# TODO — languages

`--language` is in the planned CLI surface ([cli.md](cli.md)); default is `ru`. Expanding the set should not change the flag surface — add a language mode (and only later a pack/`install <code>` if assets appear).

## Priority

**Super-priority: Russian with English insertions.**

`--language ru` means:

- primary transcript language is Russian
- English product / library / API names are the usual *canonical* targets
- Cyrillic or mangled Latin variants map **to** the dictionary’s canonical form (often Latin / mixed-case English)
- do **not** translate full sentences Russian ↔ English
- do **not** invent names missing from shipping lexicon / `--terms`

Example handoff:

```text
мы используем гитхаб экшенс
        ↓ vd-fix-terms
мы используем GitHub Actions
```

(Recognition repair `гитхап` → `гитхаб` is `vd-fix-asr`.)

## Shipping languages

| Code | Status | Notes |
|------|--------|-------|
| `ru` | default / first to ship | Russian + English insertions; canonical forms often EN product names |
| `en` | reserved | EN-primary transcripts — later |
| `de` | reserved | Later |
| `auto` | reserved | Detect from artifact / heuristic; prefer `ru` fallback for this product |

## Near term (when implementing)

- [ ] Accept `ru` at CLI / config; unknown codes → exit 2.
- [ ] Shipping lexicon: common tech names for `ru` mode.
- [ ] `--terms` load + merge (CLI overrides shipping).
- [ ] Fixtures: Cyrillic / mangled variants → canonical.
- [ ] Unit / e2e: no invent; structure untouched; no presentation / ASR jobs.
- [ ] Decide later whether optional `install` / packs are needed.

## Later

| Code | Status | Notes |
|------|--------|-------|
| `en` | TODO | EN-primary; still dictionary-only |
| `de` | TODO | Catalog reserved |
| `auto` | TODO | Heuristic; resolve to pack (`ru` fallback) |

Add rows here when a new language is requested — do not invent CLI flags per language.

## Per-language work (repeat for each new code)

- [ ] Shipping (or pack) lexicon for that language mode.
- [ ] Fixtures under `fixtures/input/` + `fixtures/expected/` + `fixtures/terms/`.
- [ ] Unit tests: only dictionary hits change; no invent / no translate.
- [ ] e2e: `run -i … --language <code> --dry-run` shows lexicon / terms paths.

## Out of scope for language work

- Translation (never — see Guarantees).
- Presentation restyle (`vd-fix-casing`).
- Open-ended ASR wording repair (`vd-fix-asr`).
- Changing artifact type or structure.
