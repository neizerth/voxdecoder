# vd-fix-terms — canonical terminology only

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI signature: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).  
Languages: [TODO-languages.md](TODO-languages.md).

**Status: implemented** (shipping lexicon + `--terms`; packs not required).

## Core rule

```text
vd-fix-terms never guesses.

Every replacement must be backed by:

- shipping lexicon;
- loaded dictionary (--terms);
- explicit rule.

Otherwise the original text is preserved.
```

`vd-fix-terms` normalizes **canonical terminology** in text artifacts to a single form from dictionaries and rules. Third step in the local cleanup pipeline:

```text
vd-fix-casing  →  vd-fix-asr  →  vd-fix-terms
 presentation       wording         terminology
```

**Rewrites only wording needed to lock terms to a canonical form.** The input artifact type and structure are preserved.

| CLI | Owns | Core rule |
|-----|------|-----------|
| `vd-fix-casing` | **presentation only** | Never changes words |
| `vd-fix-asr` | **wording only** | Changes words only to restore meaning |
| `vd-fix-terms` | **canonical terminology only** | Never guesses |

---

## Priority language

**Default and shipping focus: Russian with English insertions** (`--language ru`).

Typical handoff from ASR repair to terms:

```text
мы используем гитхап экшенс
        ↓ vd-fix-asr
мы используем гитхаб экшенс
        ↓ vd-fix-terms
мы используем GitHub Actions
```

`vd-fix-asr` restores recognition. `vd-fix-terms` locks the project-canonical spelling / casing / spacing of the term.

---

## Quick start

```bash
vd-fix-terms run -i meeting.txt
vd-fix-terms run -i meeting.txt --terms ./assets
vd-fix-terms run -i meeting.txt --terms ./assets --terms ./extra.yaml
vd-fix-terms run -i meeting.txt --terms ./corp.yaml --no-shipping-lexicon
vd-fix-terms run -i meeting.txt --progress=json --dry-run
```

A **shipping lexicon** for common tech terminology is expected so `run` works without files. Project knowledge via repeatable `--terms` — prefer `vd-assets` assets directory (`./assets`). Convert Office/PDF with `vd-assets` first. Optional `install` packs remain a possible future — do not force install before `run`.

---

## Why its own binary

Canonical terminology is a different job from presentation and recognition repair:

| Topic | vd-fix-terms | Not here |
|-------|----------------|----------|
| Changes | Product / library / API / protocol / format names to one form | Punctuation / layout (`vd-fix-casing`) |
| Scope | How the project spells it | What ASR misheard (`vd-fix-asr`) |
| Authority | Dictionaries + rules only — **no guessing** | Free-form LM rewrite |
| Output type | Same as input | Never restyles into another format |
| Sources | `--terms` (repeatable), shipping lexicon, optional packs later | Audio / re-transcription |

Shared I/O via [`crates/`](../../../crates/) (do not fork artifact/output/progress here). No presentation or ASR-repair flags “because another binary has them”.

---

## Behavior

**Fixes**

- product names
- libraries / frameworks
- APIs
- protocols
- file formats
- companies
- project names
- abbreviations
- English identifiers (when the dictionary says so)

**Uses**

- **shipping lexicon** — common tech vocabulary shipped with the binary
- **`--terms`** — prefer `vd-assets` output (`./assets`); also glossary files (**repeatable**)
- rules that map known variants → one canonical form
- optional future: installable term packs (domain / language)

**Does not**

- restyle / reformat the text (`vd-fix-casing`)
- repair ASR mishearings that are not already dictionary variants (`vd-fix-asr`)
- invent canonical forms that are not in a loaded source
- translate the transcript
- rewrite sentences for style or brevity
- re-run ASR on audio

**Examples**

```text
кубернетис     →  Kubernetes
кубернетес     →  Kubernetes
си плюс плюс   →  C++
чат джипити    →  ChatGPT
рест апи       →  REST API
сейф тензорс   →  SafeTensors
гитхаб экшенс  →  GitHub Actions
джи сон        →  JSON
ямл            →  YAML
```

**End of pipeline**

```text
Мы обсуждали кубернетес и сейф тензорс.
        ↓ vd-fix-terms
Мы обсуждали Kubernetes и SafeTensors.
```

---

## Sources precedence

Highest priority first:

1. **`--terms` (CLI)** — left → right; **last wins** on the same variant
2. **user config** (optional future: default terms paths; same last-wins within that list)
3. **shipping lexicon** (unless disabled, e.g. `--no-shipping-lexicon`)

```text
--terms a.yaml --terms b.yaml
→ b overrides a on shared variants
→ both override shipping
```

Lower sources never invent a replacement that a higher source already defined differently.

---

## Glossary shape (illustrative)

Inside a `vd-assets` bundle the shared on-disk name is **`terms.yml`** (also accepted as a standalone `--terms` file). Exact schema is still an implementation detail ([cli.md](cli.md)). Product-shaped entry example:

```yaml
canonical: Kubernetes
variants:
  - k8s
  - кубернетис
  - кубернетес
  - kubernetes

---

canonical: GitHub Actions
variants:
  - github actions
  - гитхаб экшенс
  - гитхап экшенс

---

canonical: JSON
variants:
  - json
  - джи сон
  - джейсон
```

Sample fixtures (when implementing): `fixtures/terms/`.

---

## Guarantees

`vd-fix-terms` never changes:

- segment boundaries
- timestamps
- speaker labels
- ids
- metadata
- artifact type / structure

It **may** change words, but **only inside transcript text spans**, and **only to forms supported by loaded dictionaries / rules**.

It never:

- invents a canonical term without a dictionary / rule entry
- applies presentation-only rewrites as its job
- repairs open-ended ASR noise as its job

**Input type == output type** (`txt→txt`, `json→json`, `srt→srt`, …). Default output stem: `.fixed.` (same for all `vd-fix-*`).

---

## Boundaries (what vd-fix-terms is not)

| Not in vd-fix-terms | Where it lives |
|---------------------|----------------|
| Punctuation / casing / whitespace | `vd-fix-casing` |
| Misheard words / sense repair | `vd-fix-asr` |
| Job queue / multi-run state | `vd-srv` |
| Re-transcription from audio | `vd-gigaam` / `vd-whisper` |

Full flag surface, progress, exit codes: [cli.md](cli.md).

---

## Public contract note

Dictionary format, matcher implementation, and any future inference backend are intentionally **outside** the public CLI contract. The CLI exposes language, term sources, progress, and the terminology job — not Candle / ONNX / engine brands or a shared multi-fix engine.
