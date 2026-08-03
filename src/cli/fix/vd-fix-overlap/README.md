# vd-fix-overlap — duplicated speech across speakers

Layout: [STRUCTURE.md](STRUCTURE.md).
CLI signature: [cli.md](cli.md).
Stack overview: [../README.md](../README.md).
ADR: [0012 — Local Cleanup: Disfluency and Overlap](../../../docs/adr/0012-local-cleanup-disfluency-and-overlap.md).

**Status: implemented.** Reads real JSON/JSONL diarized artifacts, detects
duplicate turns, and — with `--apply` — removes them and writes a fixed
artifact. Built on a new shared primitive in `vd-artifact`
([`segments.rs`](../../../crates/vd-artifact/src/segments.rs)): see
[STRUCTURE.md](STRUCTURE.md) "Structural mutation: resolved" for the design.

## Core rule

```text
Never delete unique speech.
Only remove duplicated content.
```

Some diarization pipelines assign the same utterance to more than one
speaker — e.g. both Speaker A and Speaker B get "Let's deploy tomorrow." —
and only one copy should remain. `vd-fix-overlap` is meeting-pipeline-only
(skipped for single-speaker transcripts):

```text
diarize → fix-overlap → meeting-merge
```

---

## What it does

`overlap::detect_duplicates(&[Utterance], &DetectOptions) -> Vec<DuplicatePair>` —
a pure, fully unit-tested function — flags pairs across **different**
speakers that are:

- **temporally close** — overlapping time ranges, or within `max_gap_ms` of
  each other (ADR 0012 §2 signals: "overlapping timestamps", "short
  temporal distance")
- **textually duplicated** — identical after normalization (`Exact`), or at
  or above `similarity_threshold` on a normalized edit-distance ratio
  (`Near`) (ADR 0012 §2 signals: "identical transcript spans",
  "near-identical transcript spans", "high lexical similarity")

Same-speaker repeats are never flagged (not diarization overlap). The
function only *recommends* — `keep`/`drop` indices — it never mutates
anything itself.

`vd-fix-overlap run` wraps this end-to-end: `vd_artifact::load` the input,
`vd_artifact::collect_segments` to get speaker+timing+text per turn, run
detection, print a report — and, with `--apply` (or any output flag),
`vd_artifact::remove_segments` the `drop` side of every pair and
`vd_artifact::write` the result.

---

## Quick start

```bash
vd-fix-overlap run -i meeting.json
vd-fix-overlap run -i meeting.json --json
vd-fix-overlap run -i meeting.json --similarity-threshold 0.9 --max-gap-ms 250
vd-fix-overlap run -i meeting.json --apply
vd-fix-overlap run -i meeting.json -o cleaned.json
```

Input shape — a JSON/JSONL turn array matching `vd-pipeline`'s
`MeetingTurn` (`{speaker, start_sec, end_sec, text}`):

```json
{
  "turns": [
    {"speaker": "A", "start_sec": 1.0, "end_sec": 3.0, "text": "Let's deploy tomorrow."},
    {"speaker": "B", "start_sec": 1.2, "end_sec": 3.2, "text": "let's deploy tomorrow"}
  ]
}
```

```text
$ vd-fix-overlap run -i meeting.json
1 candidate duplicate pair(s) found (2 turns checked):
  [exact] keep=0 (A) drop=1 (B) similarity=1.00

$ vd-fix-overlap run -i meeting.json --apply -q
$ cat meeting.fixed.json   # Speaker B's duplicate turn is gone
```

---

## Never modifies

Per ADR 0012 §2: speaker order, timestamps, or any field on a turn other
than `drop`'s text, and never touches a turn below both thresholds — "if
uncertain, both copies are preserved." A `drop` turn is either deleted
whole or, when it deterministically contains `keep`'s text plus a unique
remainder (ADR 0012 §2 "partial duplicates"), rewritten to just that
remainder — see [STRUCTURE.md](STRUCTURE.md) for exactly when trim vs.
remove applies.

---

## Boundaries (what vd-fix-overlap is not)

| Not in vd-fix-overlap | Where it lives |
|-------------------------|-----------------|
| Speech disfluencies (fillers, false starts) | `vd-fix-disfluency` |
| Misrecognized words / homophones | `vd-fix-asr` |
| Diarization itself (who spoke when) | `vd-diarize` |
| Combining per-speaker transcripts into one meeting doc | `vd-meeting` |
| Trimming a mid-text shared fragment (not at a clean prefix/suffix boundary) | not implemented — see [STRUCTURE.md](STRUCTURE.md) non-goals |

Full flag surface, exit codes: [cli.md](cli.md).
