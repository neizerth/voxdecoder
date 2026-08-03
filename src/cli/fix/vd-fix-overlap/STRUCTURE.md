# vd-fix-overlap — project layout

Rust crate for the diarization-overlap duplicate-speech **remover**.

**Status: implemented.** Workspace member: `src/cli/fix/vd-fix-overlap`.
Reads real JSON/JSONL diarized artifacts, detects duplicate turns, and — with
`--apply` — removes them and writes a fixed artifact. See "Structural
mutation: resolved" below for how.

Related: [README.md](README.md) (product notes) · [cli.md](cli.md) (flags) · [ADR 0012](../../../docs/adr/0012-local-cleanup-disfluency-and-overlap.md)

---

## Structural mutation: resolved

ADR 0012 §2 asks `vd-fix-overlap` to **remove a whole diarized segment**
when it duplicates another speaker's speech — a *structural* change
(delete a segment), not a text-only edit inside one span, unlike every
other `vd-fix-*` CLI (which only ever rewrites `TextSpan::text` in place).

This crate originally shipped detection-only because `vd-artifact` had no
primitive for that (documented at length in git history / ADR 0010). That
gap is now closed: `vd-artifact` gained
[`segments.rs`](../../../crates/vd-artifact/src/segments.rs), exposing:

```rust
pub struct SegmentId(pub u32); // separate numbering scheme from TextSpan's SpanId

pub struct Segment {
    pub id: SegmentId,
    pub speaker: Option<String>,
    pub start_sec: Option<f64>,
    pub end_sec: Option<f64>,
    pub text: String,
}

pub fn collect_segments(artifact: &Artifact) -> Vec<Segment>;
pub fn remove_segments(artifact: &mut Artifact, ids: &[SegmentId]) -> usize;
pub fn set_segment_text(artifact: &mut Artifact, id: SegmentId, text: &str) -> bool;
```

Scope, deliberately narrow:

- **JSON/JSONL only.** A "segment" is any array-element object (or bare
  JSONL line) carrying a recognized text key (`TEXT_KEYS`, shared with
  `text_spans.rs`) — matching `vd-pipeline`'s `MeetingTurn { speaker,
  start_sec, end_sec, text }` shape. `speaker`/`start_sec`/`end_sec` are
  read from a small recognized-key list (case-insensitive), `None` if
  absent.
- **`Txt`/`Md`/`Srt`/`Vtt` stay untouched** — all three mutators are a no-op
  and `collect_segments` returns empty for them. `Txt`/`Md` are single-span
  (no notion of multiple turns); `Srt`/`Vtt` carry timing but no structural
  speaker field, so cross-speaker duplication can't be verified for them.
- **Read is a snapshot** (`Segment` owns `String`s, no `&mut` anywhere) —
  the two mutators are narrow and specific: `remove_segments` only ever
  deletes whole matched array elements/lines (nothing else on survivors
  changes); `set_segment_text` only ever overwrites one segment's text
  field, never its speaker/timing or any other segment.

This is the same "one sanctioned handle, everything else unreachable"
discipline `TextSpan` already uses for text-only edits — `segments.rs` is a
second, narrower set of handles for the operations (`vd-fix-overlap`) that
genuinely need more than text.

`vd-fix-overlap` uses all three directly in `cli/run.rs`: `collect_segments`
feeds `overlap::detect_duplicates` (converting `start_sec`/`end_sec` seconds
to the detector's millisecond `Utterance`), and — gated by `--apply` (or any
output flag) — each detected pair's `TrimAction` decides whether `drop` is
deleted via `remove_segments` or rewritten via `set_segment_text`, before
`vd_artifact::write` serializes the result.

---

## Tree

```
src/cli/fix/vd-fix-overlap/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md                 # this file
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── types.rs                 # re-exports overlap::{Utterance, DetectOptions, ...}
│   ├── paths.rs                 # VD_FIX_OVERLAP_* via vd_artifact::paths
│   ├── cli/
│   │   ├── mod.rs               # -i/-o/-d/--in-place/--overwrite/--apply/--json/-q
│   │   ├── run.rs               # load → collect_segments → detect → (apply: remove_segments + write)
│   │   └── config_cmd.rs
│   ├── config/
│   │   ├── mod.rs               # similarity_threshold / max_gap_ms, CLI > config > default
│   │   └── file.rs
│   └── overlap/                 # the detection logic — pure, no I/O
│       ├── mod.rs
│       └── detect.rs
│
├── tests/
│   ├── unit/
│   │   ├── mod.rs
│   │   ├── cli.rs
│   │   └── detect.rs            # exact dup, near dup, time-gating, same-speaker exclusion
│   └── e2e/
│       ├── mod.rs
│       └── binary.rs            # real JSON I/O, --apply write, --json report, exit codes
│
└── fixtures/
    └── input/turns.json
```

---

## Domain model

```rust
/// One diarized utterance: speaker + time range (ms) + text. This crate's
/// own type, built from `vd_artifact::Segment` (seconds → ms) — detection
/// works in whole milliseconds.
pub struct Utterance {
    pub speaker: String,
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

pub struct DetectOptions {
    pub similarity_threshold: f64, // [0.0, 1.0], default 0.85
    pub max_gap_ms: u64,           // default 500
}

pub enum DuplicateKind { Exact, Near }

/// What `--apply` does to `drop`'s turn (ADR 0012 §2 "partial duplicates").
pub enum TrimAction {
    RemoveWhole,
    TrimTo(String), // drop contains keep as a clean prefix/suffix + unique remainder
}

/// A recommendation, not a mutation. `keep` / `drop` are indices into the
/// input slice.
pub struct DuplicatePair {
    pub keep: usize,
    pub drop: usize,
    pub kind: DuplicateKind,
    pub similarity: f64,
    pub trim: TrimAction,
}

pub fn detect_duplicates(utterances: &[Utterance], opts: &DetectOptions) -> Vec<DuplicatePair>;
```

### Detection algorithm (`overlap/detect.rs`)

For every pair `(i, j)` with `i < j`:

1. Skip if `utterances[i].speaker == utterances[j].speaker` (same-speaker
   repetition is not diarization overlap).
2. Skip unless temporally close: time ranges overlap, **or** the gap
   between the closer edges is `<= max_gap_ms`.
3. Normalize both texts (lowercase, alphanumeric-only, whitespace
   collapsed). Skip if either is empty after normalization.
4. Exact match → `DuplicateKind::Exact`, `similarity = 1.0`. Otherwise
   compute a normalized-Levenshtein similarity ratio; flag as
   `DuplicateKind::Near` only if `>= similarity_threshold`.
5. `keep` = the earlier-starting utterance (index tie-break); `drop` = the
   other.
6. `trim` = `TrimTo(remainder)` when `drop`'s *original* text (case-folded
   only, not the aggressively-stripped comparison key from step 3) starts
   or ends with `keep`'s text plus a genuine remainder — i.e. `drop` is a
   strict superset of `keep`. Otherwise `RemoveWhole`. This check is
   independent of `similarity_threshold`/`DuplicateKind` — it only ever
   promotes an already-qualifying pair from delete-whole to trim, it can't
   make a non-qualifying pair get flagged.

`detect_duplicates` itself stays a pure function over slices — it only
*recommends*; `cli/run.rs` is the only place that turns a pair's `trim`
into an actual `remove_segments`/`set_segment_text` call.

---

## Modules

| Path | Role |
|------|------|
| `overlap/` | Detection + trim-decision algorithm — pure, fully tested, no I/O |
| `cli/` | UX from [cli.md](cli.md); `run.rs` owns all I/O — real `vd_artifact::load`/`write`, `collect_segments`/`remove_segments`/`set_segment_text` |
| `config/` | Persist + merge `similarity_threshold` / `max_gap_ms` |
| `types.rs` | Re-export `overlap::*` |
| `paths.rs` | `VD_FIX_OVERLAP_CONFIG` via `vd_artifact::paths::config_path` |

---

## Non-goals

`vd-fix-overlap` intentionally does **not**:

- fix disfluencies (`vd-fix-disfluency`), wording (`vd-fix-asr`), or
  anything else text-level
- run diarization itself (`vd-diarize`)
- guess at a fix when confidence is low — a pair either clears both
  thresholds or is not reported; there is no partial/uncertain output
- trim anything beyond a clean, deterministic prefix/suffix containment —
  a fuzzy near-match with the shared fragment in the *middle* of the text
  (not at a clean boundary) always gets `RemoveWhole`, never a partial
  trim; finding and excising a mid-string shared fragment safely is a
  distinct, harder problem this crate doesn't attempt
- operate on `Txt`/`Md`/`Srt`/`Vtt` — no structural speaker field to verify
  cross-speaker duplication (see "Structural mutation: resolved" above)

---

## Tests

All tests under `tests/` — **no** `#[cfg(test)]` in `src/`.

| Path | Role |
|------|------|
| `tests/unit/detect.rs` | Exact duplicate, near-duplicate via edit distance, below-threshold exclusion, overlapping vs. non-overlapping-in-time, same-speaker exclusion, keep/drop ordering, custom thresholds |
| `tests/unit/cli.rs` | clap parsing / validation |
| `tests/e2e/binary.rs` | end-to-end `run` against real JSON fixtures, `--apply` write + `-o`/`--in-place`, `--json` report shape, config roundtrip, exit codes |
| `src/crates/vd-artifact/tests/unit/artifact_segments.rs` | `collect_segments`/`remove_segments` primitive itself — JSON/JSONL/bare-line shapes, no-op for Txt/Md/Srt/Vtt, structure preservation on removal |

```bash
cargo test -p vd-fix-overlap --test unit
cargo test -p vd-fix-overlap --test e2e
cargo test -p vd-artifact --test unit
```

---

## Build

```bash
cd src/cli/fix/vd-fix-overlap
cargo build --release
cargo test
cargo run -- run -i fixtures/input/turns.json
cargo run -- run -i fixtures/input/turns.json --apply -o cleaned.json
```

Binary name: `vd-fix-overlap`. Workspace member:
`src/cli/fix/vd-fix-overlap` — depends on `vd-artifact` (load/write/segments)
and `vd-output` (`-o`/`-d`/`--in-place`/`--overwrite` resolution).

---

## `vd-pipeline` / `vd-meeting` wiring

`Capability::FixOverlap` is wired in: `vd-pipeline/src/exec/subprocess.rs`
dispatches it via a dedicated `run_fix_overlap` (not the generic `run_fix`
every other `vd-fix-*` uses — this one needs `--apply` added, or the
report-only default would produce no output file). `vd-meeting`'s graph
builder (`planner/graph/mod.rs::build_job`) appends a `fix-overlap` step
**after** `meeting-merge`, not between `diarize` and `meeting-merge` as
ADR 0012's original text said — `diarize` alone produces no text, only a
timing-only `SpeakerTimeline`; the combined speaker+timestamp+text shape
this crate needs only exists once `meeting-merge` has run. See ADR 0012's
Decision for the full correction. The step only appears for diarized
(multi-speaker) meetings and rewrites the same well-known meeting-artifact
path `meeting-merge` just wrote, so downstream consumers of that filename
see the deduplicated result without needing to know a new file exists.
