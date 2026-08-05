# vd-classify

Filename-based meeting input classification heuristics — implements ADR 0017 Decision H.

Single source of truth for classification rules documented in `skills/vd-meeting/skill.md`:

- **Filename noise stripping**: timestamps, separators → clean names
- **Mix detection**: keyword tokens ("mix", "merged", "весь", "общ") → `Role::Room`
- **Gender inference**: Russian/English name tables → optional `Gender::Male | Female`

## Public API

```rust
pub fn strip_basename_noise(stem: &str) -> String
pub fn is_mix_token(name: &str) -> bool
pub fn infer_gender(given_name: &str) -> Option<Gender>
pub fn classify_inputs(paths: &[PathBuf]) -> Vec<ClassifiedInput>
```

## Consumers

- **vd-meeting --interactive** (ADR 0017 Decision D): calls `classify_inputs` in-process to propose meeting input roles before showing menu.
- **vd-srv plan.classify** (Runtime API): gateway for `vd-mcp` `classify_meeting_inputs` MCP tool. Skill calls the tool instead of re-hardcoding heuristics.

## Design notes

- Pure functions, no I/O. Tested directly.
- Gender: returns `None` (never guesses) on ambiguous/unisex names like "Alex", "Саша", "Женя".
- Mix detection: substring containment (not word-boundary match) — accepts false positives ("общая_запись" matches "общ" prefix) since results are human-confirmed, never auto-applied.
- Codec: Cyrillic script/casing preserved (never transliterated to Latin slugs).
