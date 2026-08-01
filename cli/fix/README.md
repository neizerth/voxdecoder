# Text cleaning CLIs

Local post-processing for transcripts and other text artifacts. Three tools, almost no overlap, one natural pipeline:

```text
vd-fix-casing  →  vd-fix-asr  →  vd-fix-terms
   (form)           (words)          (canonical names)
```

| CLI | Changes | Spec |
|-----|---------|------|
| `vd-fix-casing` | Presentation only | [vd-fix-casing/](vd-fix-casing/) ([cli](vd-fix-casing/cli.md)) |
| `vd-fix-asr` | Words / meaning | TBD |
| `vd-fix-terms` | Canonical names | TBD |

Queue / background runs: [`vd-srv`](../vd-srv/).

---

## Shared contract

All three CLIs share the same I/O contract so they can be chained in any order (recommended order above):

- Accept **any text artifact**: `txt`, `json`, `jsonl`, `srt`, `vtt`, `md`, and `vd-*` native artifacts.
- **Input type == output type** (`txt→txt`, `json→json`, `srt→srt`, …).
- Default output: `{stem}.fixed.{ext}` (never `.cased.` / `.clean.`).
- Shared UX: `run` / `config`, `--dry-run`, `--progress=json`, `--language`, priority CLI > config > default.

Each binary documents an explicit **Guarantees** section (what it never changes). That contract is more important than the option list: it makes the pipeline safe to chain.

How they differ is only **Behavior**:

| CLI | Behavior |
|-----|----------|
| `vd-fix-casing` | changes presentation only |
| `vd-fix-asr` | changes words only |
| `vd-fix-terms` | changes canonical term representation only |

---

## `vd-fix-asr`

Fixes speech-recognition errors.

**Fixes**

- misrecognized words
- homophones
- Russian / English mix-ups
- technical terms distorted by ASR
- obvious errors that break meaning

**May use**

- a local language model
- extra documents
- a project glossary
- user dictionaries
- neighboring segments for context

**Does not**

- restyle / reformat the text (that is `vd-fix-casing`)
- force terminology to a canonical form (that is `vd-fix-terms`)

**Example**

```text
Мы обсуждали кубернетис и сейфтензорс.
        ↓ vd-fix-asr
Мы обсуждали кубернетес и сейф тензорс.
```

(Only clear recognition mistakes are corrected here.)

---

## `vd-fix-terms`

Normalizes terminology to a single canonical form.

Works from dictionaries and rules — it does not “guess.”

**Fixes**

- product names
- libraries
- APIs
- companies
- project names
- abbreviations
- English identifiers

**Examples**

```text
кубернетис   →  Kubernetes
си плюс плюс →  C++
чат джипити  →  ChatGPT
рест апи     →  REST API
```

**Sources**

- `terms.yaml` / `terms.json`
- Markdown / README / docs
- user glossary

**Example (end of pipeline)**

```text
Мы обсуждали кубернетес и сейф тензорс.
        ↓ vd-fix-terms
Мы обсуждали Kubernetes и SafeTensors.
```

---

## Why this split

Each tool owns one layer of the text:

1. **Form** — make it readable (`vd-fix-casing`)
2. **Sense** — fix what was misheard (`vd-fix-asr`)
3. **Terms** — lock names to the project vocabulary (`vd-fix-terms`)

Together: **presentation → meaning → terminology**. That covers almost all local transcript cleanup while keeping each binary small, clear, and independent.
