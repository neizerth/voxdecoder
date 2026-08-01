# TODO — languages

`--language` is in the CLI surface ([cli.md](cli.md)); default is `ru`. Expanding the set should not change the flag surface — add a language mode (and only later a pack/`install <code>` if assets appear).

## Priority

**Super-priority: Russian with English insertions.**

`--language ru` means:

- primary transcript language is Russian
- English words, identifiers, library/product fragments, and code-switching are expected
- repair restores what was *said*, including English tokens when ASR mangled them
- do **not** translate Russian → English or English → Russian
- do **not** lock English product names to a project-canonical form (that is `vd-fix-terms`)

Example target mix:

```text
мы используем гитхап экшенс в стейджинге
        ↓ vd-fix-asr
мы используем гитхаб экшенс в стейджинге
```

(`GitHub Actions` canonicalization → `vd-fix-terms`.)

## Shipping languages

| Code | Status | Notes |
|------|--------|-------|
| `ru` | default / first to ship | Russian + English insertions |
| `en` | reserved | Pure / EN-primary transcripts — later |
| `de` | reserved | Later |
| `auto` | reserved | Detect from artifact / heuristic; prefer `ru` fallback for this product |

## Near term

- [x] Accept `ru` at CLI / config; unknown codes → exit 2.
- [x] Fixtures: Russian ASR noise + English technical insertions.
- [x] Wire `language` into `AsrFixer::load`.
- [x] `--context` materials + neighbor window.
- [x] Unit / e2e coverage for `ru` wording repair.
- [ ] Decide later whether optional `install` / packs are needed.

## Later

| Code | Status | Notes |
|------|--------|-------|
| `en` | TODO | EN-primary; Russian insertions optional |
| `de` | TODO | Catalog reserved (`shipping = false`) |
| `auto` | TODO | Heuristic; resolve to pack (`ru` fallback) |

Add rows here when a new language is requested — do not invent CLI flags per language.

## Per-language work (repeat for each new code)

- [ ] Catalog entry + embedded (or remote) pack assets.
- [ ] Repair rules / backend assets for that language mode.
- [ ] Fixtures under `fixtures/input/` + `fixtures/expected/` tagged by language.
- [ ] Unit tests: meaning restored; structure untouched; no translate / no terms-locking.
- [ ] e2e: `run -i … --language <code> --dry-run` shows `model` / pack status.

## Out of scope for language work

- Translation (never — see Guarantees).
- Presentation restyle (`vd-fix-casing`).
- Canonical term normalization (`vd-fix-terms`).
- Changing artifact type or structure.
