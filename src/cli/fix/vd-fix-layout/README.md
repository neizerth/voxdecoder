# vd-fix-layout — text layout only

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI signature: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).  
Languages beyond `ru` / `en`: [TODO-languages.md](TODO-languages.md).

**Status: implemented.** Workspace member; capability `fix-layout` in default Job after `fix-terms`.

## Core rule

```text
Never changes lexical content.

Only whitespace and paragraph / block boundaries may change.
The input artifact type is preserved.
Does not rewrite, translate, or “improve” wording.
```

This is the **primary guarantee** of `vd-fix-layout`: words, numbers, names, and terms stay as they are — only presentation of structure may change.

`vd-fix-layout` is a **local**, language-specialized layout fixer for **readable long-form text** — transcripts, summaries, notes, and other prose artifacts.

```text
v1 implements paragraph layout only.

Future versions may add additional
layout transformations while preserving
the core guarantee:

Never changes lexical content.
```

Later layout work may include long-turn splits, list-safe whitespace, quote-block preservation, and similar — still without changing lexical content. The CLI name stays `vd-fix-layout`.

| CLI | Owns | Core rule |
|-----|------|-----------|
| `vd-fix-casing` | presentation (punct, case, quotes) | Never changes words |
| `vd-fix-asr` | wording / meaning | Changes words only to restore meaning |
| `vd-fix-terms` | canonical terminology | Never guesses |
| `vd-fix-layout` | **layout / block structure** | **Never changes lexical content** |

---

## Product bet: RU and EN first-class, local only

```text
Local tooling only.

No cloud LLM.
No remote “rewrite this chapter” API.
Shipping language packs: ru / en.
```

| Language | Role | Expectation |
|----------|------|-------------|
| `ru` | shipping | Discourse markers, connective patterns, pause cues tuned for Russian |
| `en` | shipping | Same class of tools for English long-form text |
| `auto` | shipping | Resolve to `ru` or `en` (see Language resolution) |
| other | out of scope | See [TODO-languages.md](TODO-languages.md) |

Each shipping language gets its **own** local pack of signals and thresholds. Packs are optional for a builtin baseline; `install ru` / `install en` can deepen models and cue lists without changing the CLI surface.

---

## Quick start

```bash
vd-fix-layout run -i meeting.fixed.txt
vd-fix-layout run -i summary.md --language auto --progress=json

vd-fix-layout install ru
vd-fix-layout install en
vd-fix-layout list
```

`run` must work without packs (embedded RU/EN baselines). `install` is optional — same UX shape as `vd-gigaam` / `vd-fix-casing`.

Models dir: platform cache, or `VD_FIX_LAYOUT_MODELS_DIR` / `--download-root` / `config set download_root`.

---

## Why its own binary

Layout is a different job from casing and wording:

| Topic | vd-fix-layout | Not here |
|-------|----------------|----------|
| Changes | Whitespace / paragraph (and future block) boundaries | Lexical content (`vd-fix-asr` / `vd-fix-terms`) |
| Scope | Readable long-form structure for `ru` / `en` | Sentence punct / casing (`vd-fix-casing`) |
| Signals | Sentences + language cues + optional **TimeMap** structural hints | Cloud summarizers |
| Output type | Same as input | Never restyles into another format |
| Packs | Optional `install ru` / `en`; builtin without install | Shared multi-fix engine |

Shared I/O via [`crates/`](../../../crates/).

---

## Pipeline place

Typical audio / meeting cleanup **before** recipes:

```text
transcribe
    ↓
fix-casing
    ↓
fix-asr
    ↓
fix-terms
    ↓
fix-layout
    ↓
postprocess
```

Layout runs **after** wording/terms so lexical fixes see continuous text, and **before** `vd-postprocess` when recipes should see already-readable paragraphs.

It also works **after** recipes on any long-form artifact:

```text
summary.md
    ↓
vd-fix-layout --language auto
```

Same for notes, export markdown, or other prose — not only raw transcripts.

---

## Behavior

Changes **layout only** (v1 = paragraphs):

- inserts paragraph breaks (`\n\n` in plain text; equivalent block boundaries in structured artifacts)
- may normalize accidental multi-blank runs
- language-aware: `--language ru` \| `en` \| `auto`
- density: `compact` \| `normal` \| `relaxed` (see Config)

Does not:

- change lexical content (words, numbers, names, terms)
- repair ASR errors
- normalize terminology
- translate
- rewrite or summarize content
- invent headings or list markup (beyond safe whitespace around existing structure)

**Example (ru)**

```text
Баня. Баня — это место, куда русские люди ходят, чтобы расслабиться. …
Самое главное — это веник. …
После бани русские любят пить чай и разговаривать.
        ↓ vd-fix-layout --language ru
Баня. Баня — это место, куда русские люди ходят, чтобы расслабиться. …

Самое главное — это веник. …

После бани русские любят пить чай и разговаривать.
```

Backend stays private. CLI sees language, density, optional TimeMap binding, pack, progress.

---

## Signals

| Signal class | Role | Notes |
|--------------|------|-------|
| Sentence boundaries | Baseline units | Prefer text already passed through `vd-fix-casing` when available |
| Language discourse cues | Topic / turn shifts | Separate cue lists for `ru` and `en` |
| **TimeMap** | Optional structural hints | Pauses, timing, speaker transitions — see below |
| Length / density policy | Avoid tiny or huge paragraphs | Via `paragraph_density`, not raw min/max sentence knobs |
| Already-layouted input | Preserve / light tidy | Do not smash deliberate `\n\n` without cause |

### TimeMap

```text
TimeMap provides optional structural hints
(pauses, timing, speaker transitions).

Layout remains fully functional
without a TimeMap.
```

Whoever authored it does not matter (`vd-preprocess`, ASR, diarize, Job / Runtime / ArtifactRef). The CLI binds a TimeMap **abstractly** (artifact / Job / Runtime) — it does not require a particular on-disk path in the product contract. Optional `--timemap` is a local convenience for standalone CLI use.

---

## Language resolution (`auto`)

When `--language auto` (or config `language = auto`):

1. Language declared on the **artifact** (if any)
2. Language associated with the bound **TimeMap** (if any)
3. **Autodetection** over text (script / lexicon heuristics → `ru` or `en`)
4. **Config** default / fallback (`ru` if still unresolved)

Resolution never invents a third shipping language.

---

## Guarantees

**Primary:**

```text
Never changes lexical content.
```

Also never changes:

- timestamps’ *values* as stored on timed units
- speaker labels
- ids
- metadata

**Structural safety:**

```text
Paragraph boundaries never split
a timed segment or speaker label.
```

Critical for JSON transcripts and Meeting Artifacts: breaks land *between* timed/speaker units, not inside them.

Only whitespace and paragraph / block boundaries may change.

**Input type == output type.** Default output stem: `.fixed.` (same for all `vd-fix-*`).

---

## Boundaries

| Not in vd-fix-layout | Where it lives |
|----------------------|----------------|
| Sentence punct / casing / quotes | `vd-fix-casing` |
| Misheard words / homophones | `vd-fix-asr` |
| Canonical product / API names | `vd-fix-terms` |
| Authoring TimeMap | `vd-preprocess` / ASR / diarize |
| Job queue | `vd-srv` |
| Recipe projection (md/json/…) | `vd-postprocess` |
| Cloud “make nice chapters” | nowhere in this stack |

---

## Config keys (planned)

| Key | Meaning |
|-----|---------|
| `language` | `ru` \| `en` \| `auto` (CLI default when unset: see resolve; config often `auto` or `ru`) |
| `download_root` | Packs / models directory |
| `paragraph_density` | `compact` \| `normal` (default) \| `relaxed` — pack maps these to internal thresholds |
| `use_timemap` | Whether to bind TimeMap structural hints when available (default: on) |
| `progress` | `text` \| `json` |

Do **not** expose low-level `min_sentences` / `max_sentences` in the public config — those stay inside the language pack / backend.
