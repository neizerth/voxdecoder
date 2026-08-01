# vd-fix-asr — wording only

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI signature: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).  
Languages: [TODO-languages.md](TODO-languages.md).

**Status: implemented (rules backend).** Packs / `install` not required.

`vd-fix-asr` repairs **speech-recognition mistakes** in text artifacts: misheard words, homophones, Russian/English mix-ups, technical terms mangled by ASR. Second step in the local cleanup pipeline:

```text
vd-fix-casing  →  vd-fix-asr  →  vd-fix-terms
 presentation       wording         canonical names
```

**Rewrites only wording needed to restore meaning.** The input artifact type and structure are preserved.

That one line is the contract:

| CLI | Owns |
|-----|------|
| `vd-fix-casing` | **presentation only** — how it is written |
| `vd-fix-asr` | **wording only** — what was said |
| `vd-fix-terms` | **canonical names only** — how the project names it |

---

## Priority language

**Default and shipping focus: Russian with English insertions** (`--language ru`).

Typical ASR error this CLI is built for (recognition fix, not canonical naming):

```text
мы используем гитхап экшенс
        ↓ vd-fix-asr
мы используем гитхаб экшенс
        ↓ vd-fix-terms
мы используем GitHub Actions
```

English identifiers and code-switched fragments stay in the language that was spoken — the job is to fix *recognition*, not to translate or to force project-canonical spellings (`vd-fix-terms`).

---

## Quick start

```bash
vd-fix-asr run -i meeting.txt
vd-fix-asr run -i meeting.txt --progress=json
vd-fix-asr run -i meeting.srt --context ./docs --context ./glossary.yaml
```

Builtin rules backend — no `install` required. Optional packs remain a possible future.

---

## Why its own binary

Wording repair is a different job from presentation and terminology:

| Topic | vd-fix-asr | Not here |
|-------|-------------|----------|
| Changes | Misrecognized words / local sense | Punctuation / layout (`vd-fix-casing`) |
| Scope | What was misheard | Canonical product names (`vd-fix-terms`) |
| Context | Neighboring segments + `--context` materials | Re-transcription from audio |
| Output type | Same as input | Never restyles into another format |
| Language focus | `ru` + English insertions first | Pure EN / DE until requested |

Shared I/O via [`crates/`](../../../crates/) (do not fork artifact/output/progress here). No presentation or terms-locking flags “because another binary has them”.

---

## Behavior

**Fixes**

- misrecognized words
- homophones
- Russian / English mix-ups
- technical terms distorted by ASR
- obvious errors that break meaning

**Uses (when available)**

- a local language model (implementation detail — not exposed as engine brand)
- neighboring segments for discourse context
- **glossary** — project terminology hints for recognition (not canonical locking)
- **dictionaries** — additional vocabulary
- **`--context`** — additional project materials (documentation, glossaries, dictionaries, source code, wiki, RFCs, …)

**Does not**

- restyle / reformat the text (`vd-fix-casing`)
- force terminology to a project-canonical form (`vd-fix-terms`)
- translate the transcript
- rewrite sentences for style or brevity
- re-run ASR on audio

**Example**

```text
мы используем гитхап экшенс для деплоя
        ↓ vd-fix-asr
мы используем гитхаб экшенс для деплоя
```

Only the recognition mistake is corrected. `GitHub Actions` as the project-canonical name is `vd-fix-terms`.

---

## Guarantees

`vd-fix-asr` never changes:

- segment boundaries
- timestamps
- speaker labels
- ids
- metadata
- artifact type / structure

It **may** change words, but **only inside transcript text spans**.

**Never invents information** that is not supported by:

- the transcript
- neighboring context
- supplied `--context` materials

It never:

- applies presentation-only rewrites as its job
- locks names to a project-canonical form

**Input type == output type** (`txt→txt`, `json→json`, `srt→srt`, …). Default output stem: `.fixed.` (same for all `vd-fix-*`).

---

## Boundaries (what vd-fix-asr is not)

| Not in vd-fix-asr | Where it lives |
|-------------------|----------------|
| Punctuation / casing / whitespace | `vd-fix-casing` |
| Canonical product / API names | `vd-fix-terms` |
| Job queue / multi-run state | `vd-srv` |
| Re-transcription from audio | `vd-gigaam` / `vd-whisper` |

Full flag surface, progress, exit codes: [cli.md](cli.md).

---

## Public contract note

Model family, inference runtime, and backend implementation are intentionally **outside** the public CLI contract. The CLI exposes language, wording repair, optional `--context`, and progress — not Candle / ONNX / llama / engine brands.
