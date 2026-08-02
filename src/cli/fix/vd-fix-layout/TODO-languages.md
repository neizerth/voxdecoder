# TODO — languages beyond `ru` / `en`

`--language` is part of the CLI from day one ([cli.md](cli.md)); default is `auto`.

**Shipping language packs:** only `ru` and `en`.  
**`auto`:** resolves to `ru` or `en` (never a third language).  
Other codes fail closed (exit 2). Do not add stub packs.

## Shipping

| Code | Status | Notes |
|------|--------|-------|
| `ru` | shipping | Builtin + optional `install ru` |
| `en` | shipping | Builtin + optional `install en` |
| `auto` | shipping (resolver) | Artifact → TimeMap → detect → config; not a pack |

## Done (near term — when implementing)

- [ ] Accept `ru`, `en`, `auto` at CLI / config; unknown codes → exit 2.
- [ ] Implement `auto` resolution order (artifact → TimeMap → detect → config).
- [ ] Wire `language` + `paragraph_density` into `LayoutFixer::load`.
- [ ] Separate backend modules / fixtures for `ru` and `en`.
- [ ] Abstract TimeMap binding + optional CLI `--timemap` convenience.
- [ ] Guarantee tests: never split timed segment or speaker label; lexical content unchanged.
- [ ] Progress phases: `loading` → `analyzing` → `layout` → `writing`.
- [ ] Pack install UX; packs optional for `run`.

## Later

| Code | Status | Notes |
|------|--------|-------|
| `de` | TODO | Only after RU/EN packs are excellent |

## Per-language work

- [ ] Catalog entry + embedded (or remote) pack assets.
- [ ] Language-specialized cues / density maps / pause defaults.
- [ ] Fixtures under `fixtures/input/<code>/` + `fixtures/expected/<code>/`.
- [ ] Lexical invariant + sensible breaks + timed-unit safety.

## Quality bar before adding a language

- [ ] Side-by-side review on real long-form text (transcripts **and** summaries).
- [ ] Builtin path usable without download.
- [ ] Pack path measurably better on held-out samples.
- [ ] No cloud dependency.

## Out of scope

- Translation.
- Lexical edits (`vd-fix-casing` / `asr` / `terms`).
- Changing artifact type.
- Cloud layout APIs.
- Public `min_sentences` / `max_sentences` knobs.
