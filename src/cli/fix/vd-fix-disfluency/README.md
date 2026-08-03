# vd-fix-disfluency — speech noise only

Layout: [STRUCTURE.md](STRUCTURE.md).
CLI signature: [cli.md](cli.md).
Stack overview: [../README.md](../README.md).
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).
Rust gates: [RUST.md](RUST.md).
ADR: [0012 — Local Cleanup: Disfluency and Overlap](../../../docs/adr/0012-local-cleanup-disfluency-and-overlap.md).

**Status: implemented (deterministic rules, scaffold-quality).** Not yet wired into `vd-pipeline`'s default job — ADR 0012 defers that to the PR that ships it (see ADR's "vd-pipeline default job" section).

## Core rule

```text
Remove speech noise.
Never remove information.
```

`vd-fix-disfluency` deterministically strips speech disfluencies from transcript text: filler syllables, repeated filler runs, empty hesitations, and (in riskier modes) false starts. Third step in the local cleanup pipeline order set by ADR 0012:

```text
transcribe → fix-asr → fix-disfluency → fix-layout → fix-terms
```

**Rewrites only disfluency noise.** The input artifact type and structure are preserved — same guarantee model as `vd-fix-asr` / `vd-fix-casing`: only `TextSpan::text` is ever touched.

| CLI | Owns | Core rule |
|-----|------|-----------|
| `vd-fix-casing` | presentation only | Never changes words |
| `vd-fix-asr` | wording only | Changes words only to restore meaning |
| `vd-fix-disfluency` | **disfluency noise only** | Removes speech noise, never removes information |
| `vd-fix-overlap` | duplicated speech across speakers | Never deletes unique speech |
| `vd-fix-terms` | canonical terminology only | Never guesses |

---

## Modes

```text
off | light | normal | aggressive
```

Default: `light`.

| Mode | Isolated fillers | Repeated filler runs | Empty hesitations | False starts |
|------|-------------------|------------------------|--------------------|---------------|
| `off` | untouched | untouched | untouched | untouched |
| `light` | removed | collapsed to **one** instance | cleaned up | untouched |
| `normal` | removed | removed entirely | cleaned up | collapsed |
| `aggressive` | removed | removed entirely | cleaned up | collapsed |

`aggressive` uses the same rule set as `normal` in this scaffold — reserved for future, riskier transforms. False starts are gated to `normal`+ because "clearly accidental" detection is inherently more likely to misfire than filler removal (ADR 0012 §1).

---

## Quick start

```bash
vd-fix-disfluency run -i meeting.txt
vd-fix-disfluency run -i meeting.txt --mode normal
vd-fix-disfluency run -i meeting.txt --progress=json
vd-fix-disfluency run -i meeting.srt --mode aggressive
```

Builtin deterministic rules — no model, no download, no `install` required.

---

## Behavior

**Removes**

- filler syllables: `эээ`, `ммм`, `эм` (ru); `um`, `uh`, `erm` (en)
- repeated filler runs: `эээ... эээ...` → one instance (`light`) or gone (`normal`+)
- empty hesitations: `Ну... эээ... да...` → `Ну, да...`
- false starts (`normal`+ only): `Я... я думаю...` → `Я думаю...`

**Never removes**

- meaningful discourse markers — hardcoded protected list (`ну да`, `ну конечно`, `вот именно`, and English equivalents) that filler / false-start rules must never touch even if they superficially match (ADR 0012 §1)
- anything outside a `TextSpan::text` — segment boundaries, timestamps, speaker labels, ids, metadata, artifact type / structure

**Example**

```text
Так, эээ... эээ... начнём. Я... я думаю, что это норм.
        ↓ vd-fix-disfluency --mode normal
Так, начнём. Я думаю, что это норм.
```

---

## Priority language

**Default: `ru`** (mirrors `vd-fix-asr`'s ru-priority default: `en` selects the English filler/discourse tables, everything else — `ru` / `de` / `auto` — resolves to the Russian tables for now).

---

## Guarantees

`vd-fix-disfluency` never changes:

- segment boundaries
- timestamps
- speaker labels
- ids
- metadata
- artifact type / structure

It **may** delete or shorten words, but **only inside transcript text spans**, and only speech noise — never unique spoken content. **Input type == output type** (`txt→txt`, `json→json`, `srt→srt`, …). Default output stem: `.fixed.` (same for all `vd-fix-*`).

---

## Boundaries (what vd-fix-disfluency is not)

| Not in vd-fix-disfluency | Where it lives |
|---------------------------|-----------------|
| Misrecognized words / homophones | `vd-fix-asr` |
| Punctuation / casing / whitespace as a job | `vd-fix-casing` |
| Paragraph / segment layout | `vd-fix-layout` |
| Canonical product / API names | `vd-fix-terms` |
| Duplicated speech across speakers (diarization overlap) | `vd-fix-overlap` |
| Re-transcription from audio | `vd-gigaam` / `vd-whisper` |

Full flag surface, progress, exit codes: [cli.md](cli.md).

---

## Public contract note

Rule tables (filler lists, protected phrases) are an implementation detail and may grow; the CLI contract is language, mode, and the disfluency-removal guarantee above — not the exact token lists.
