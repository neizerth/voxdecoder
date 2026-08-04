# ADR 0016 — Participant-Grounded Meeting Assembly

**Status:** Partially implemented — planner room purposes (no meeting `trim-silence`), fix-overlap gate, mix subtract + residual attribution (never label `room`), timeline load bridge, stub overlaps; prefer-active speaker in detect; markdown sidecar regenerated after fix-overlap; word/term packs landing with ADR 0014  
**Type:** ADR  
**Date:** 2026-08-03

**Related:**

- [`vd-meeting`](../../src/cli/process/vd-meeting/) · [`vd-diarize`](../../src/cli/process/vd-diarize/) · [`vd-fix-overlap`](../../src/cli/fix/vd-fix-overlap/)
- [ADR 0012 — fix-disfluency / fix-overlap](0012-local-cleanup-disfluency-and-overlap.md)
- [ADR 0014 — orphan letters / fillers](0014-orphan-letters-and-filler-cleanup.md)
- [ADR 0015 — HTTP job artifacts](0015-http-job-artifacts-endpoint.md)

---

## Motivation

When a meeting has **clean per-participant tracks** plus a **room mix**, the same speech often appears twice:

1. Correctly on the participant ASR track.
2. As bleed / mix ASR echo on another track or on the room transcript.

`vd-fix-overlap` already removes cross-speaker near-duplicate turns, but:

- it ran only when diarization was enabled;
- room + participants defaulted room to **timeline only** (no mix ASR), so there was nothing to “subtract from”;
- diarize `overlaps[]` stayed empty and the timeline was often not applied to text.

Product rule:

```text
Clean participant tracks are the source of truth for those speakers' words.
The mix supplies who/when (diarize) and residual speech not covered by tracks.
No duplicate wording in the final meeting artifact.
```

---

## Decision

### 1. Room purposes when participants exist

Default room purposes become:

```text
[transcript, timeline]
```

(previously `[timeline]` only). Room ASR produces a mix transcript used for residual speech after subtract. Diarize still runs on room audio for the speaker timeline.

### 2. Text-domain subtract (not audio)

After per-track ASR (+ fix-\*) and room ASR:

```text
mix_residual = mix_turns − spans covered by any participant turn
               (time overlap ∧ high lexical similarity)
```

- Participant turns are kept as-is.
- Mix contributes only **uncovered** residual (no dedicated mic / unknown voice).
- No audio echo-cancellation in this ADR.

### 3. Diarize overlaps → interruptions / prefer-active

`vd-diarize` populates `overlaps[]` (regions where multiple speakers are active).

When two participant turns are near-duplicate in time:

- prefer the speaker that the timeline marks as active;
- drop or trim the bleed copy on the other track.

`fix-overlap` runs whenever there are **≥ 2 text sources** (not only when diarize is on).

### 4. Artifact class boundaries (cleanup)

| Class | Owner |
|-------|--------|
| Speech noise / glued onset / fillers | `vd-fix-disfluency` (+ `vd-text`, ADR 0014) |
| Misheard words (when listed) | `vd-fix-asr` via `--dictionary` / project `.voxdecoder/asr-dictionary.yml` / context materials — **no in-code builtin table** |
| Product / stack names | `vd-fix-terms` via `--terms` / meeting `docs` — **no shipping word list in the binary** |
| Cross-speaker duplicate turns | `vd-fix-overlap` + subtract |
| Residual rare mishears | optional later AI cleanup |

---

## Pipeline sketch

```text
participant tracks  → ASR → fix-*  ─┐
room mix            → ASR → fix-*  ─┼→ subtract → meeting-merge → fix-overlap → meeting.json
room mix            → diarize (timeline + overlaps) ─┘
```

Order note (ADR 0012 correction stands): merge (or subtract+merge) before `fix-overlap`, because overlap needs speaker + time + text together.

After `fix-overlap` rewrites `meeting.json`, the pipeline **must** regenerate the sibling `meeting.md` from the surviving turns. Merge writes both artifacts before dedupe; leaving the pre-dedupe markdown would keep cross-speaker bleed in the human-readable export even when JSON is clean.

---

## Non-goals

- Audio-domain echo cancel / beamforming
- Mid-string fuzzy deletion without clean boundaries
- LLM cleanup
- Inventing unknown brand names without a dictionary / glossary

---

## Success criteria

- Room + N participants: room has both transcript and timeline by default.
- Mix residual has no near-copy of participant wording in overlapping windows.
- Mix residual speaker labels are **participant names** (via diarize correlation or fallback) — never the branch id `room`.
- Meeting preprocess **omits `trim-silence`**: current TimeMap is uniform-only; silenceremove would linearly stretch compacted speech and break subtract / speaker clocks. Piecewise silence maps are future work.
- Same-window bleed across two participant tracks collapses to one speaker (prefer timeline when present).
- `fix-overlap` present in the Job whenever ≥ 2 text branches exist.
- After `fix-overlap`, sibling `meeting.md` matches deduped JSON turns (no stale pre-dedupe markdown).
- Diarize artifact can expose non-empty `overlaps[]` when the backend detects them.
