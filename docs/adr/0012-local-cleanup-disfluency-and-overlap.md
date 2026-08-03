# ADR 0012 — New Local Cleanup Capabilities: `vd-fix-disfluency` + `vd-fix-overlap`

**Status:** Implemented — both crates shipped and wired into `vd-pipeline`/`vd-meeting` (see Decision)  
**Type:** ADR / architectural RFC  
**Date:** 2026-08-03

**Related:**

- [`vd-fix-asr`](../../src/cli/fix/vd-fix-asr/) · [ADR 0010](0010-vd-fix-asr-local-transcript-cleanup.md)
- [`vd-fix-layout`](../../src/cli/fix/vd-fix-layout/)
- [`vd-pipeline`](../../src/cli/process/vd-pipeline/) — default job builder: [`job/default.rs`](../../src/cli/process/vd-pipeline/src/job/default.rs)
- [`vd-meeting`](../../skills/vd-meeting/)
- [ADR 0013 — vd-text shared linguistic infrastructure RFC](0013-local-linguistic-infrastructure.md)

---

## Motivation

ASR quality is already high. The remaining quality issues are often caused not by incorrect recognition, but by characteristics of spontaneous speech and speaker overlap. These issues can be improved deterministically without using LLMs — same local-first philosophy as [ADR 0010](0010-vd-fix-asr-local-transcript-cleanup.md).

Two new local cleanup capabilities are proposed.

---

## 1. `vd-fix-disfluency`

### Goal

Remove obvious speech disfluencies while preserving the speaker's meaning. The output must still represent what was said.

### Core rule

```text
Remove speech noise.
Never remove information.
```

### Examples

**Filler syllables** — `эээ`, `ммм`, `эм` → removed.

**Repeated filler syllables** — `эээ... эээ...` → `ээ...` (or removed, depending on mode).

**Empty hesitations** — `Ну... эээ... да...` → `Ну, да...`

**False starts** — `Я... я думаю...` → `Я думаю...` (only when clearly accidental).

### Never remove

Meaningful discourse markers that carry semantic meaning: `Ну да.`, `Ну конечно.`, `Вот именно.`

### Modes

```text
off | light | normal | aggressive
```

Default: `light`.

### Config

```yaml
remove_fillers: true
mode: light
```

### Pipeline position

```text
transcribe → fix-asr → fix-disfluency → fix-layout → fix-terms
```

---

## 2. `vd-fix-overlap`

### Goal

Remove duplicated speech introduced by diarization overlap.

### Core rule

```text
Never delete unique speech.
Only remove duplicated content.
```

### Motivation

Some diarization pipelines assign the same utterance to multiple speakers, e.g. both Speaker A and Speaker B get "Let's deploy tomorrow." — only one copy should remain.

### Detection signals

- identical transcript spans
- near-identical transcript spans
- overlapping timestamps
- high lexical similarity
- short temporal distance

### Safe corrections

- **Exact duplicates**: same line from A and B → keep only one copy.
- **Partial duplicates**: A says "Deploy tomorrow morning.", B says "Tomorrow morning." → trim the duplicated fragment when deterministic.
- **Repeated overlap fragments**: both mention "...the backend..." mid-sentence → keep only one occurrence.

### Never modify

- speaker order
- timestamps
- speaker identities
- unique additions made by either speaker

### Confidence

Corrections require high confidence. If uncertain: preserve both copies.

### Pipeline position

Meeting pipeline only:

```text
diarize → fix-overlap → meeting-merge
```

Skipped for single-speaker transcripts.

---

## Capability independence

These capabilities are independent. Projects may enable `fix-asr` + `fix-disfluency` + `fix-layout` without overlap removal. Meeting pipelines may additionally enable `fix-overlap`.

---

## Future compatibility

Both capabilities stay deterministic. Future AI cleanup should operate after them:

```text
ASR → fix-asr → fix-disfluency → fix-overlap → fix-layout → fix-terms → (optional) AI cleanup
```

The local pipeline should resolve the vast majority of mechanical transcript artifacts before any generative model is involved.

---

## Success criteria

- `vd-fix-disfluency` removes speech noise without changing meaning.
- Multiple cleanup modes are supported.
- `vd-fix-overlap` removes duplicated speech introduced by diarization.
- Both capabilities are deterministic.
- Both are reusable by `vd-pipeline` and `vd-meeting`.
- Neither capability requires an LLM.

---

## `vd-pipeline` / `vd-meeting` wiring

`vd-pipeline`'s default job (`job/default.rs::default_job`) is a **static, executable** step list keyed on the `Capability` enum — every default `vd-pipeline` run executes exactly this list against real binaries. `Capability::FixDisfluency` is now in it:

```text
preprocess → transcribe → prepare-context → fix-casing → fix-asr → fix-disfluency → fix-terms → fix-layout
```

`fix-overlap` is meeting-pipeline-only. **Correction to this ADR's original placement**: the text above originally said "diarize → fix-overlap → meeting-merge", but `diarize` alone produces a timing-only `SpeakerTimeline` with **no text** (confirmed in `vd-diarize`'s output shape and `vd-meeting`'s graph builder) — the combined speaker+timestamp+text shape `fix-overlap` needs only exists *after* `meeting-merge` runs. Actual wiring, in `vd-meeting/src/planner/graph/mod.rs::build_job`:

```text
diarize → (per-branch transcripts) → meeting-merge → fix-overlap → (Job output)
```

`fix-overlap` is only appended when `want_diarize` is true (single-speaker meetings have nothing to dedup), rewrites the exact same well-known meeting-artifact path `meeting-merge` just wrote (not a new `.fixed.` file, so downstream consumers looking for `meeting_<date>_<participants>.json` still find the deduplicated version), and runs with `--apply` (`vd-pipeline`'s dispatcher needs a dedicated `run_fix_overlap` — the generic `run_fix` helper used by every other `vd-fix-*` capability doesn't pass `--apply`, and without it `vd-fix-overlap` only reports, it doesn't write).

---

## Decision

**Implemented.**

- **`vd-fix-disfluency`** (`src/cli/fix/vd-fix-disfluency/`): full implementation per §1 — filler removal, repeated-filler-run collapsing, the empty-hesitation composite rule, false-start collapsing (gated to `normal`/`aggressive`), protected-phrase guard, `off|light|normal|aggressive` modes. 33 tests, clippy clean.
- **`vd-fix-overlap`** (`src/cli/fix/vd-fix-overlap/`): full implementation per §2, including real artifact I/O and partial-duplicate trimming — see below. 33 tests (14 e2e + 19 unit), clippy clean.
- **Structural-mutation gap closed**: `vd-fix-overlap` originally shipped detection-only because `vd-artifact` had no primitive for reading speaker+timestamp+text together or mutating a segment. `vd-artifact` gained `segments.rs` (`collect_segments`/`remove_segments`/`set_segment_text`/`Segment`/`SegmentId`), scoped to JSON/JSONL array-of-turn-object shapes matching `vd-pipeline`'s `MeetingTurn` — the same "one sanctioned handle" discipline `TextSpan` already uses for text-only edits, just a second, narrower set of handles for the operations that genuinely need more. `Txt`/`Md`/`Srt`/`Vtt` are unaffected (no structural speaker field, so all three functions are empty/no-op for them). 11 new tests in `vd-artifact/tests/unit/artifact_segments.rs`; full workspace test suite (`cargo test --workspace`) passes with zero regressions in every other `vd-artifact` consumer.
- **Partial-duplicate trimming implemented** (ADR §2 "partial duplicates"): `overlap::detect_duplicates` now also computes a `TrimAction` per pair — `RemoveWhole` (unchanged default), or `TrimTo(remainder)` when `drop`'s original text case-insensitively contains `keep`'s text as a clean prefix or suffix plus a genuine unique remainder (a deterministic containment check, independent of the fuzzy edit-distance similarity gate). `--apply` now calls `set_segment_text` instead of `remove_segments` for `TrimTo` pairs, so a longer `drop` turn's unique content survives instead of being destroyed. A fuzzy near-match with the shared fragment in the *middle* of the text (no clean boundary) still always gets `RemoveWhole` — mid-string fragment excision is out of scope (see `STRUCTURE.md` non-goals).
- `vd-fix-overlap run` now: loads a real artifact, detects duplicates, and — with `--apply` (or any output flag: `-o`/`-d`/`--in-place`) — applies each pair's `TrimAction` (remove or trim) and writes a fixed artifact via the same `-o`/`-d`/`--in-place`/`--overwrite` convention every other `vd-fix-*` CLI uses. Without `--apply`, it stays report-only (no write), matching ADR 0012's "if uncertain, preserve both copies" posture. `--json`'s report now includes an `action` (`"remove"`/`"trim"`) and, for trims, `trimmed_text`.
- **`vd-pipeline`/`vd-meeting` wired**: `Capability::FixDisfluency` and `Capability::FixOverlap` added to the schema; `default_job()` now includes `fix-disfluency` between `fix-asr` and `fix-terms`; `vd-meeting`'s graph builder appends a `fix-overlap` step after `meeting-merge` (not between diarize and merge — see the placement correction above) when the meeting is diarized. All exhaustive `match Capability` sites across `vd-pipeline` updated; `default.yaml`/`default.json` fixtures and their `job_parse`/`dry_run` tests updated to the new 8-step default job; two new `vd-meeting` integration tests assert `fix-overlap` is present (and rewrites the same output path as `meeting-merge`) for diarized meetings and absent for single-speaker ones. Full `cargo test --workspace` (105 test binaries) passes.
