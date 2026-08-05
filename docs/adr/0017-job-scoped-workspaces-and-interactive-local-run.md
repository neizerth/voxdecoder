# ADR 0017 — Job-Scoped Workspaces & Interactive Local Run for `vd-meeting` / `vd-pipeline`

**Status:** Accepted
**Type:** ADR
**Date:** 2026-08-05

**Related:** [`vdctl`](../../src/cli/manage/vdctl/) · [`vd-meeting`](../../src/cli/process/vd-meeting/) · [`vd-pipeline`](../../src/cli/process/vd-pipeline/) · [`vd-srv`](../../src/cli/manage/vd-srv/) (Job Store: [`store/mod.rs`](../../src/cli/manage/vd-srv/src/store/mod.rs), paths: [`paths.rs`](../../src/cli/manage/vd-srv/src/paths.rs)) · [`vd-artifact`](../../src/crates/vd-artifact/) · [`skills/vd-meeting`](../../skills/vd-meeting/skill.md) · [`skills/vd-audio`](../../skills/vd-audio/skill.md) · [ADR 0008](0008-input-resolution-layer.md) · [ADR 0009](0009-skills-packaging-and-distribution.md) · [ADR 0015](0015-http-job-artifacts-endpoint.md) · [ADR 0016](0016-participant-grounded-meeting-assembly.md)

---

## Motivation

Today there are two ways to run a meeting/audio Job:

1. **AI agent + Skill** (`skills/vd-meeting`, `skills/vd-audio`) via MCP `process_meeting` / `process_audio`. These Skills are rich: filename heuristics classify mix vs. per-speaker tracks, infer gender, look for accompanying docs, ask the user to confirm/edit via `AskUserQuestion`, and offer diarization/cleanup choices.
2. **Direct CLI** (`vd-meeting run`, `vd-pipeline run`). These are mechanical: every input needs an explicit `--input role=…,path=…`, `--context`/`--docs` needs an explicit path, there is no mix/gender detection, and no confirmation step.

A human at a terminal (or a script, or an agent without MCP wired up) only gets path 2 today and loses all the UX that makes path 1 usable. `vdctl` cannot fill that gap — see **Decision A**. Separately, intermediate files for both paths collide across repeated/parallel runs on the same input, which is exactly why the Skills need a "Prior run / leftovers" overwrite-vs-continue prompt.

---

## Problem

### 1. Intermediates are not job-scoped

`vd_artifact::paths::work_dir_for_input` resolves intermediates to a single fixed location per input file:

```text
{input_parent}/.voxdecoder/work/
```

Every step (`vd-preprocess`, `vd-gigaam`, `vd-assets` prepare-context, diarize) writes into that same flat directory regardless of which run produced it. Two runs on the same input — retry, reprocess with different `speed`, parallel meeting + solo audio pass — overwrite or collide with each other's intermediates. This is why Skills must ask "overwrite vs. continue" before every `execute: true`: the storage layer gives them no isolation to reason about.

Contrast with `vd-srv`'s own Job Store (`src/cli/manage/vd-srv/src/store/mod.rs`), which already scopes bookkeeping (events, artifact index, logs) per `JobRecord.id` under `$VD_HOME/jobs/{id}/`. That isolation exists for Runtime metadata but **not** for the actual media/transcript intermediates the Executor produces — those still land in the flat, unscoped `work/` dir next to the source file. The `Job` document itself (`vd-pipeline::job::schema::Job`) has no `id` field at all — only `vd-srv` mints one, and only for Runtime-submitted Jobs.

### 2. `vdctl` is not, and should not become, a Job runner

`vdctl` (`src/cli/manage/vdctl/README.md`) has an explicit, already-documented Golden Rule:

```text
Everything that modifies the platform  → vdctl
Everything that processes user data    → the Runtime
```

`cli.md` lists `run · submit · transcribe · pipeline · service` under **Non-commands**. `vdctl api <method>` exists but is documented as an **Operator passthrough for debug/status only** (`job.status …`), not a supported way to start Jobs — it has no filename heuristics, no confirmation flow, no polling loop, none of what makes the Skill usable.

Meanwhile `vd-meeting` and `vd-pipeline` already are dedicated, local, synchronous processing CLIs — they call the shared `vd-pipeline` Executor in-process/via subprocess, need no `vd-srv` running, and are exactly the layer the "Runtime" side of the Golden Rule points to. **They are the correct place for this capability, not `vdctl`.**

### 3. No filename-based classification / confirmation outside the AI Skill

`skills/vd-meeting/skill.md` already documents working heuristics (section **Filename heuristics**): strip timestamps/whitespace from the basename to get a name; detect mix/merged tokens (`mix`, `mixed`, `merged`, `all`, `room`, `full`, `combined`, `common`, `overall`, `together`, `весь`, `общ`, `микс`, `слит`, `полный`); infer gender from given names; and a **Mix + tracks** choice (diarize on mix / align to mix / tracks only) that defaults to diarizing when a mix is present, opt-out. None of this is available to `vd-meeting run`.

### 4. No `context/` folder convention outside the AI Skill

Both Skills accept `docs` / `role: context` and note it feeds `vd-assets` → `fix-asr` / `fix-terms`, but resolution is always "the user told us the path." `vd-pipeline`'s default Job hardcodes `docs` to `"."` (`src/cli/process/vd-pipeline/src/job/default.rs:163`) when unset — there is no convention of looking for a `context/` subfolder next to the media or the project.

---

## Goal

1. Move processing **intermediates** out of per-project folders into one global, content-addressed cache — resuming interrupted work and deduplicating repeat runs, without changing where final deliverables (`meeting_*.md`, `*.fixed.txt`, …) land.
2. Give `vd-meeting run` / `vd-pipeline run` an **interactive mode** with the same filename classification, gender inference, mix detection, and confirmation UX the Skills already have — implemented once, in Rust, shared with the Skill instead of re-specified as prose.
3. Auto-detect a `context/` folder next to the media (or project) in both interactive and non-interactive runs.
4. Bound the growth of job-scoped work directories with a prune command.
5. Do all of this **without** adding processing responsibilities to `vdctl`.

---

## Decision

### A. `vdctl` stays a platform manager; no `run`/`submit` verb is added to it

Re-affirms the existing Golden Rule and Non-commands list — this ADR does not touch `vdctl`. New capability lands in `vd-meeting` and `vd-pipeline`, which already own local Job execution. `vdctl api <method>` remains debug-only passthrough; it is not upgraded into a supported entry point for this feature.

### B. One global, content-addressed cache — not per-project folders

Revises the original per-project `voxdecoder/{job_id}/` idea: intermediates move to **one shared location**, keyed so identical work is automatically found and reused instead of merely isolated.

```text
$VD_HOME/cache/{key}/
```

`$VD_HOME` is the existing platform data root (`vdctl::paths::home_dir()` — OS `ProjectDirs` data dir, override via `VD_HOME`), already the parent of `models/`, `skills/`, `bundles/`; `cache/` becomes a sibling. This reading of "a global parent folder next to the executables" is `$VD_HOME` rather than the literal install/bin directory, because `$VD_HOME` is already the one platform-data root every binary agrees on (Workspace and Installed modes resolve it the same way) and it is guaranteed writable — the bin directory is not, in an Installed layout. Resolution lives in `vd_artifact::paths` (new `cache_dir()`, alongside today's `project_dir()` / `work_dir_for_input()`) so both local CLI runs and `vd-srv` reach it without a new cross-crate dependency — `vd-pipeline` and `vd-meeting` already depend on `vd-artifact`.

**Cache key, two schemes:**

| Job shape | Key | Why |
|---|---|---|
| Single input (`vd-pipeline` / audio) | Content hash of the resolved input file (BLAKE3, full file) | One input, no ambiguity — same bytes anywhere, any project, any time → same cache slot, fully automatic dedup. Hashing cost is negligible next to multi-minute ASR wall time. |
| Multi-input (`vd-meeting` / meeting) | The run's `job_id` | A meeting is defined by a whole request (roles, participants, diarization/alignment mode, speed, …) — fingerprinting that is fragile and easy to get subtly wrong (a fingerprint that ignores a field that actually changes output silently reuses stale results). A `job_id` the caller already holds is unambiguous instead. |

`job_id` is `vd-srv`'s existing `JobRecord.id` (`store::mod.rs::new_job_id`) when running under the Runtime; for local `vd-meeting run` with no Runtime involved, the CLI mints one with the same generator (moved to a shared spot so the format matches: `job-{nanos:x}-{pid:x}`). This id is already what Skills track for `get_job` / `cancel_job` — no new identifier for the agent to remember.

**Resume, not just isolate:** interrupted meeting transcription is resumed by re-running with the **same `job_id`** (the Skill/CLI already has it from the original submission). The Executor's existing reuse logic (`vd-pipeline::exec::subprocess::reuse_existing`, today keyed on a fixed output path) starts from `$VD_HOME/cache/{job_id}/` instead, finds already-completed step outputs, and only re-runs what is missing — no design change to that logic, just relocation of where it looks. Audio jobs get the same property for free: rerun on the same file (same hash) and the cache is already warm.

**Startup check (CLI and Skill alike):** before any `execute: true` / real run, resolve the cache key and check `$VD_HOME/cache/{key}/` for existing artifacts. If present, offer the same **overwrite vs. continue** choice the Skills already make in **Prior run / leftovers** — same UX, just resolved against the global cache instead of a per-input flat dir. No manifest file is needed: the key **is** the lookup (hash for audio; `job_id` — already known to the caller — for meetings), so there is nothing extra to keep in sync or go stale.

**Final deliverables are unaffected** — `meeting_*.md`/`.json`, fixed transcripts, and `.clean.md` siblings still land at `output_dir` / `working_dir` root on completion, exactly as today (the Skills' **Artifact output location** contract is unchanged). The cache also naturally retains a copy of the final artifact as one of the DAG's outputs, which is a useful side benefit for recovering a deliverable the user deleted from `output_dir` by hand — not a primary goal here, just noted.

This directly satisfies "не плодить доп папки": nothing is written next to the user's media or project files anymore. `.voxdecoder/` keeps its original, narrower meaning — persistent project assets (`md/`, `terms.yml`, `asr-dictionary.yml`) that `vd-assets` / `vd-fix-*` already read from there — untouched by this decision.

### D. Interactive run mode for `vd-meeting run`

Add `--interactive`, auto-enabled on a TTY when no `--input` flags are given and a directory/glob of files is passed instead (e.g. `vd-meeting run ./recordings/`). The same command run non-interactively (`--input role=…` given explicitly, piped/non-TTY, or `--json`) skips the wizard and behaves exactly as today — this is additive, chosen over an always-explicit `--interactive`-only flag so a human at a terminal gets the good default without needing to know the flag exists.

1. List candidate media files in the given directory.
2. Per file: strip timestamp-looking tokens and extra whitespace from the basename → candidate name; check mix/merged tokens → propose `role: room`; otherwise propose `role: participant` with the cleaned name as `participant` id, and infer gender from the given name where confident.
3. Print the proposed classification as a table and prompt: accept all / edit an entry / drop an entry (plain terminal prompts — no MCP `AskUserQuestion` here, so a simple numbered-menu readline loop).
4. If a room/mix file was accepted, default to **diarize on mix** with an explicit `[Y/n]` to opt out.
5. Proceed to the existing confirm-and-run flow.

Classification logic itself (step 2) is not reimplemented here in CLI-specific code — see **Decision H**.

### E. Same wizard for plain audio via `vd-pipeline run`

There is no separate `vd-audio` binary — `skills/vd-audio` already maps 1:1 onto `vd-pipeline`'s default Job. Rather than add a new CLI, extend `vd-pipeline run -i <file-or-dir>` with the same context-folder auto-detect (**F**) and a lighter interactive confirmation (single file: confirm/edit inferred name + speed + output path; no mix/participant classification since `vd-pipeline` is single-track). Keeps one binary per responsibility instead of proliferating CLIs.

### F. `context/` folder auto-detection — always on

Resolution order for `--context` (`vd-meeting`) / `--docs` (`vd-pipeline`) when not passed explicitly, in **both** interactive and non-interactive runs:

1. `{media_dir}/context/` if it exists.
2. `{project_dir}/context/` if it exists and differs from `{media_dir}`.
3. Otherwise, today's default (`.`) — unchanged.

No prompt gates this — it is pure convention-over-configuration, additive, and cannot break a script that has no `context/` folder (falls through to the same `.` it gets today). A non-interactive run that picks up `context/` prints one line to stderr (`using context: ./context (auto-detected)`) so it is never silent about what it fed the fixers. An explicit `--context`/`--docs` always overrides auto-detection.

Update `skills/vd-meeting/skill.md` and `skills/vd-audio/skill.md` **Accompanying documents** sections to mention the `context/` convention so AI agents and the CLI agree on the same default instead of the Skill always asking.

### G. Retention: `prune` command for the global cache

`$VD_HOME/cache/` now grows unbounded — every distinct audio hash and every meeting `job_id` keeps its own copy of prepared media until something removes it, and because the cache is global (shared across every project on the machine) it accumulates faster than the old per-project dirs would have. Add `prune [--older-than <duration>] [--keep-recent <n>] [--dry-run]` (defaults: `--older-than 14d`, always `--dry-run` unless `--yes` is also passed — cache entries carry no back-reference to "am I the current attempt" the way a per-project `last.json` would have, so pruning is pure age/size-based, not correctness-aware).

Ownership is `vd-pipeline` (+ `vd-meeting prune` alias for discoverability), not `vdctl`: the data physically lives under `$VD_HOME`, but its *content* is Job/Executor intermediates — the same category **Decision A** keeps off `vdctl` for execution, and that reasoning is kept unbroken rather than carved out an exception for cache housekeeping. `vdctl doctor` / `vdctl paths` may still *report* `$VD_HOME/cache/` size as a read-only diagnostic — it already walks `$VD_HOME` for `models/`/`skills/`/`bundles/` — it just never deletes from it.

### H. Shared classification crate + MCP tool (token-reduction for the Skill)

Filename/gender/mix heuristics currently live as **prose** in `skills/vd-meeting/skill.md` (the **Filename heuristics** / **Gender** / **Mix + tracks** sections) — the agent re-derives the classification itself every conversation, which is both a duplicated implementation (Skill prose vs. whatever **Decision D** builds in Rust) and a real chunk of the Skill's token budget on every load.

Extract the heuristics into a new shared crate, `vd-classify` (`src/crates/vd-classify/`): timestamp/whitespace stripping, mix-token table, gender-inference table — pure functions, no I/O, unit-testable directly (today's only spec is Skill prose; that becomes test fixtures instead).

Consumers:

- `vd-meeting --interactive` (**Decision D**) calls it in-process.
- `vd-srv` exposes a new Runtime API method (e.g. `plan.classify`) that calls the same crate; `vd-mcp` gateways it as a new MCP tool (e.g. `classify_meeting_inputs`) — consistent with `vd-mcp` never inventing logic itself (`README.md` §Skills & AI integration: "Gateway only").
- The Skill is rewritten to **call the tool** and present its suggestions via `AskUserQuestion` instead of re-deriving the rules from prose. The **Filename heuristics** / **Gender** / **Mix + tracks** sections shrink to "call `classify_meeting_inputs`, confirm the result with the user" — the token-heavy rule tables move into `vd-classify`'s doc-comments/tests, read once by whoever edits the crate, not by the agent on every turn.

This is the same pattern ADR 0008 already established for input resolution (`vd-input` as the one shared implementation consumed by CLI and Runtime alike) — applied to classification instead of resolution.

---

## Consequences

**Positive**

- Nothing is written next to the user's media or project files anymore — no `voxdecoder/`, no `.voxdecoder/work/` litter, addressing "не плодить доп папки" directly.
- Retries and audio dedup automatically instead of merely not-colliding: same file content anywhere on the machine reuses the same cache slot; interrupted meetings resume by re-submitting the same `job_id` instead of starting over.
- No new manifest / index file to keep in sync — the cache key (hash or `job_id`) *is* the lookup, so there is nothing that can go stale independent of the cache itself.
- `vdctl`'s boundary stays intact and documented — no scope creep into Job execution, no rewrite of its Golden Rule.
- Terminal/script users get the same classification quality AI Skill users already have, instead of hand-writing `--input role=…,path=…` for every file.
- `context/` becomes a discoverable convention instead of a silent `.` default, in every mode.
- Classification logic has exactly one implementation (`vd-classify`) instead of Rust-in-CLI plus prose-in-Skill drifting apart; the Skill gets shorter and cheaper to load per turn.

**Trade-offs**

- The cache is global and shared across every project on the machine — a real benefit for dedup, but it also means one project's re-run can be served by intermediates another, unrelated project produced from the same audio file. Should be harmless (content-addressed, byte-identical input → byte-identical intermediate) but is a behavior change worth calling out explicitly, not something users would expect from "processing intermediates" today.
- Meeting resume depends on the caller retaining and re-supplying the same `job_id` — if it's lost (chat history cleared, script didn't save it), the interrupted work is not found and reprocessing starts fresh. No fallback discovery path is proposed here (e.g. "list meetings that look like this input set") — flagged as a possible follow-up, not solved by this ADR.
- Full-file BLAKE3 hashing on every audio run adds I/O + CPU proportional to file size; negligible next to ASR wall time for normal recordings, worth re-checking against very large inputs (multi-hour video) before calling this a non-issue.
- Interactive terminal prompting is new UI surface in two binaries that were previously pure "flags in, exit code out" tools — needs its own tests (non-TTY behavior must stay deterministic).
- New crate (`vd-classify`) plus a new Runtime API method and MCP tool is more surface than doing the heuristics as CLI-only code — justified by removing the duplicate prose spec and shrinking the Skill, but it is the largest single piece of this ADR.
- Global cache is a single point of contention if two processes race on the same key concurrently — needs a lock/atomic-write story (e.g. write into `{key}.tmp-{pid}/` and rename into place) so a crashed writer never leaves a half-written cache entry that a later "resume" trusts as complete.

---

## Success criteria

- [ ] Nothing lands next to user media/project files — no `.voxdecoder/work/`, no `voxdecoder/` sibling folder.
- [ ] Two audio runs on byte-identical file content (regardless of project/path) hit the same `$VD_HOME/cache/{hash}/` and reuse completed intermediates.
- [ ] Re-running `vd-meeting` with the same `job_id` after an interruption resumes from cached step outputs instead of redoing completed work.
- [ ] Final deliverable paths (`meeting_*.md`, fixed transcripts) unchanged from today.
- [ ] `vd-meeting run ./dir` in interactive mode (auto-triggered on TTY) reproduces the Skill's mix/gender/name classification on a fixture set, via `vd-classify`.
- [ ] `context/` auto-detected next to media when present, in interactive **and** non-interactive runs; explicit flag still wins.
- [ ] `vd-pipeline prune` (+ `vd-meeting prune` alias) exists, defaults to a dry-run-safe posture; `vdctl doctor`/`paths` may report cache size but never deletes.
- [ ] `skills/vd-meeting/skill.md`'s Filename heuristics / Gender / Mix + tracks sections replaced by a call to `classify_meeting_inputs`; token size of the Skill drops measurably.
- [ ] `vdctl` surface (`cli.md`, README, non-commands list) unchanged.

## Implementation status

Phase 0 (P0-1..6) and Phase 1 (P1-A..G) are done and merged — cache primitives, `new_job_id()` relocation, cache-path conversion across all `subprocess.rs` call sites, the `vd-pipeline::interactive` menu primitive, `vd-classify` (implemented: `strip_basename_noise`, `is_mix_token`, `infer_gender`, `classify_inputs` — all with unit tests), `plan.classify` + `classify_meeting_inputs` MCP tool, `resolve_context_dir()`, CLI scaffolding for `--interactive` on both `vd-meeting run` and `vd-pipeline run`, the `vd-pipeline prune` subcommand, and the `skill.md` rewrite to call the classify tool.

**Known gaps found during review** (do not treat as done despite CLI surface existing):

1. `vd-meeting`'s `interactive::show_wizard()` (`src/cli/process/vd-meeting/src/interactive/mod.rs`) is `todo!()` — the `--interactive` flag parses and auto-triggers on TTY but calling it panics.
2. `vd-pipeline run --interactive` (`src/cli/process/vd-pipeline/src/cli/run.rs`) parses the flag into `RunArgs.interactive` but nothing reads that field — it is currently inert, not wired to anything.
3. P1-H (resume test) has no actual test in the repo. A first attempt (`tests/integration/resume.rs`) didn't compile against the real `default_job`/`resolve_job`/`Executor` signatures and was deleted rather than fixed.
4. `prune` filters candidates by directory mtime only — it does not check whether a `.tmp-{pid}` entry's PID is a still-live process before offering it for deletion.
5. P2-1 (end-to-end `vd-meeting run` on cache), P2-2 (concurrency/crash-safety test for atomic cache write) are not started.

### Remaining tasks (H1–H5)

Each task's definition of done includes unit tests for any new non-trivial logic — a task is not complete on `cargo build`/existing-test-sweep success alone. Review each task's diff before starting the next; do not batch.

**H1 — `vd-meeting` interactive wizard body** (closes gap 1)
File: `src/cli/process/vd-meeting/src/interactive/mod.rs`. Implement `show_wizard()`:
1. Collect candidate files from `working_dir` (or an explicit path list).
2. Call `vd_classify::classify_inputs(&paths)`.
3. Build `Vec<MenuItem<InputSource>>` from the result (label = `"{role} {name} [{gender}]"`).
4. Run through `vd_pipeline::interactive::run()` (accept/edit/drop loop) on stdin/stdout.
5. `edit_one` callback: allow manual override via a small `role=…,name=…,gender=…` text format.
6. Call `crate::paths::resolve_context_dir()`; if found, offer to add it as `role: context` (separate y/n).
7. Return `(Vec<InputSource>, Option<PathBuf>)`.

Tests required: edit-string parsing (valid/invalid), `MenuItem` construction from `ClassifiedInput`, context-offer branch (found/not found), full happy path with a mocked `Cursor` stdin (mirror the pattern in `vd_pipeline::interactive::run`'s own tests).

**H2 — Wire `vd-pipeline run --interactive`** (closes gap 2)
File: `src/cli/process/vd-pipeline/src/cli/run.rs`. When `interactive && !dry_run && input.is_some()`: classify the single input via `vd_classify::classify_inputs`, confirm/edit through `vd_pipeline::interactive::run()` before building the default Job. Non-interactive path must be unchanged.
Tests required: CLI flag parsing (`--interactive`/`--non-interactive` conflict — already validated, add a test asserting it), unit test confirming the non-interactive path is untouched.

**H3 — Real resume test** (closes gap 3)
Before writing anything, read the current signatures of `default_job()`, `resolve_job()`, and `Executor` in `src/job/default.rs`, `src/job/resolve.rs`, `src/exec/mod.rs` — do not guess from memory. Build a Job from the same input file twice (same content-hash → same `cache_key`), run the preprocess step, assert the second run reuses the cached file in `$VD_HOME/cache/{key}/` (mtime unchanged / subprocess not re-invoked).

**H4 — Prune: skip live-process cache entries** (closes gap 4)
File: `src/cli/process/vd-pipeline/src/cli/prune.rs`. Detect `{key}.tmp-{pid}` entries; before offering one for deletion, check whether `pid` is a live process (`kill(pid, 0)` or `sysinfo`) — skip if alive.
Tests required: mtime-based filtering (extend existing), pid-alive detection (fake/mock pid), orphaned tmp-dir (dead pid) always prunable.

**H5 — Concurrency/crash-safety test for atomic write** (P2-2)
Test `vd_artifact::atomic_temp_path` + `finalize_atomic`: two writers targeting the same `cache_key` concurrently must not produce a partially-written final file. Simulate a crash (skip `finalize_atomic`) and assert the orphaned `.tmp-{pid}` is invisible at the final path.

### H1–H5 status: done, two review passes applied

Implemented and merged. First review pass (4 findings: stderr/stdout flush mismatch in H1, dead if/else branch in H2, double `read_dir` in H4, weak hash assertion in H3) fixed. A second, independent review pass (fresh `/model` switch, no shared context with the pass that wrote the code) found five more issues — all fixed:

1. **Critical, confirmed by compiled repro**: H4's `is_pid_alive()` on macOS checked `/dev/fd/{pid}` — the *current* process's own open file descriptors, unrelated to whether a process with that PID exists. On macOS (this project's dev platform) this made prune treat almost every live job as dead, so `--force` could delete an in-progress job's cache directory. A test that would have caught this (`current_pid_should_be_alive`) had failed and been *deleted* rather than the implementation fixed. Corrected to shell out to `kill -0 {pid}` (this workspace forbids `unsafe`, so no direct libc FFI) — POSIX-uniform across Linux/macOS/BSD. The real-PID test is back and passing.
2. H5's concurrency test had both racing threads compute the *same* tmp path, because `atomic_temp_path()` keys on `std::process::id()` which is identical for both threads in one test process — it wasn't exercising the two-distinct-writers race the atomic-rename design protects against. Rewritten with two literal distinct tmp siblings (as two real OS processes would produce) plus a separate crash-safety test (writer dies before `finalize_atomic`; `final_path` must stay invisible).
3. H3's `resume.rs` only asserted `resolve_job()` produces equal `cache_key` strings for equal content — it never proved a second run's step lookup actually lands in the *same on-disk directory* a first run wrote to. Added a test that writes a marker file via one resolution's `job_cache_dir(cache_key)` and reads it back via an independently-computed second resolution's path.
4. **The `resume.rs` file lived at the repo root (`tests/integration/resume.rs`)**, but the root `Cargo.toml` is a pure `[workspace]` manifest with no `[package]` — cargo never discovers tests there. All three resume tests, across both review passes, had never actually run. Moved into `vd-pipeline`'s existing `tests/integration/` (which does have a wired `[[test]]` target) as a new `resume` module; verified with a real `cargo test` run showing `3 passed`.
5. H2's two "tests" constructed a `RunArgs` literal and asserted on the exact fields just set — tautological, couldn't fail regardless of whether `execute()`'s interactive wiring worked. Replaced with tests that call the real `execute()` on a temp file and assert the `--interactive --dry-run` guard actually completes without touching stdin (this also caught an unrelated bug: the test fixture's `asr: "whisper"` doesn't exist as an engine — `TranscribeEngine::parse` only recognizes `gigaam`; fixed the fixture, not the parser).

Also fixed in this pass: an unused `Cursor` import left in `vd-meeting`'s interactive module (compiler warning, harmless but sloppy).
