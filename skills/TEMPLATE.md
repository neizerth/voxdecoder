# Skill title

## Purpose

One short paragraph: what this Skill does and which Runtime domain it covers.

**Language:** This Skill is written in English. Reply to the user in their language (or the agent's configured conversation language). Do not switch user-facing messages to English just because this document is English.

## Workflow

Numbered steps for the agent (confirm inputs, call tools, then follow the Runtime Contract).

When the domain accepts accompanying documents (glossaries, agendas, PDFs), collect paths and pass them to the Runtime (`docs` on audio Jobs, or `role: context` on meeting inputs) so `vd-assets` can build project terms — do not paste file contents into chat instead.

When prior intermediates exist next to the media (`*.prepared.*`, `*.fixed.txt`, `meeting_*.md`, …), ask **overwrite vs continue** (Choices UX) before `execute: true`. Pass `overwrite: true` only for a fresh reprocess.

Set artifact output to the **conversation project root** (Claude / Cursor / Claude Code project folder): **required** absolute `working_dir` / `output_dir` / `output.dir`. Omitting them dumps into Runtime `.` (= vdctl workspace, often VoxDecoder) — forbidden when the chat project is elsewhere. Media paths may stay absolute elsewhere.

After a successful transcript / meeting artifact, Skills may offer **optional conservative AI cleanup** (opt-in only; never automatic). Identical contract in `vd-audio` / `vd-meeting` — [ADR 0011](../docs/adr/0011-conservative-ai-transcript-cleanup-in-skills.md). When the user opts in, run a **single tight pass** (one read, one write, all strategies together; no transcript paste into chat). Recommended noise strips trailing `Угу`/`Ага`, collapses echo invites (`Давай, давай`), clears empty husks, and эканье/аканье/stutter runs; **keep** sole-turn `Угу.`; mid-sentence `типа`/`как бы` stay optional. **Never** rewrite speaker labels — and **never** invent `room` / role ids as `**Name**` headers. After `.clean.md`, verify label set ⊆ source; fail → discard and redo.

## Runtime Contract

This Skill starts long-running Runtime Jobs.

### Execution

- Start with `<submit_tool>` (e.g. `process_audio`, `process_meeting`).
- The tool returns a Job `id` (also called `job_id`).

### Progress

- Use `get_job` with that `id` to monitor execution until `completed`, `failed`, or `cancelled`.
- When reporting status to the user, include **all** of these from `get_job` when present: `progress`, `phase`, `processed`, `total`, `unit`. Do not omit counters from the status line.
- Do not use HTTP polling (`curl`, `/health`, `/jobs/…`).
- Do not inspect Runtime sockets directly.

**Ballpark duration** (wall clock, local Metal / CPU; wide variance):

- Short audio (≈1–5 min media): often **1–5 minutes** of Job time.
- Longer media / meetings: often **several minutes to tens of minutes**.
- Preprocess `speed` (1.5× / 2.0× / 2.2×) shortens ASR wall time; tell the user the estimate is rough.

**Polling interval (adaptive — never spam):**

- **Forbidden:** polling every 10s / 15s / 30s. Floor is **60s**; default is **90s**.
- Compute next wait from the latest `get_job`:
  - If `unit` is `chunk` and `total` is set:  
    `remaining = total - processed` (missing `processed` → 0)  
    `interval_sec = clamp(90, 300, remaining * 45)`
  - If counters are absent: **90s**.
  - Near the end (`progress` ≥ 90): **60–90s**.
- Prefer MCP `get_job` + wakeup/schedule. No shell poll loops. Never name a shell variable `status` (zsh readonly) — use `job_status` / `st`.

**Liveness (do not treat as stuck):**

- Overall `progress` may stay at **0%** (or flat) for a long time while a single step runs (especially `transcribe`).
- If `processed`/`total` (or `phase`) **moves** between polls, the Job is alive — keep waiting; do not escalate, cancel, or restart.
- Only escalate after **several** checks (each ≥ the adaptive interval) where **none** of `progress`, `phase`, `processed`, or `total` changed and there is no Runtime error.

### Results

- When the Job reaches `completed`, use `list_artifacts` with the Job `id` to discover outputs.

### Cancellation

- If the user asks to stop processing, call `cancel_job` with the Job `id`.
- Do not start a second Job when cancellation is requested.

### Failures

- If the Job fails, report the Runtime error from `get_job`.
- Do not retry automatically unless the user explicitly asks.
- Classify connectivity / MCP / Job failures per **Recovery** below — do not treat every error as “start the Runtime”.

### Recovery

If the Runtime cannot be reached (e.g. `connection refused`, `runtime unavailable`, `cannot connect to vd-srv`, `health → unavailable`):

- Ask the user whether they want to start it.
- Suggest:

```text
vdctl ensure
```

(`vdctl ensure` starts the Runtime only if it is not already running. Prefer it over unconditional `vdctl up`.)

If MCP tools are unavailable (e.g. `No MCP server configured`, `Tool not found`, `process_audio does not exist`):

- Do not suggest `vdctl up` or `vdctl ensure`.
- Ask the user to install or verify the MCP integration.
- Suggest:

```text
vdctl mcp verify
```

or

```text
vdctl mcp install
```

If a Job fails with Metal / GPU resource errors (e.g. `Failed to create metal resource: Buffer`):

- Prefer **retry** — ASR may auto-retry on CPU after Metal buffer failures.
- If it still fails: set `device: "cpu"` and re-run.

If a Job is already running / the Runtime answers:

- Use Runtime tools (`get_job`, `cancel_job`, `list_artifacts`).
- Never poll Runtime over HTTP.
- Do not suggest lifecycle commands.

## Examples

```text
<submit_tool> → job_id → get_job (until completed) → list_artifacts
<submit_tool> → job_id → cancel_job
```

## Notes

Reply in the user's / agent's conversation language (see Language above).

Optional caveats (confirmation before `execute: true`, …).

---

Official layout (ADR 0005):

```text
Skill
├── Purpose
├── Workflow
├── Runtime Contract   (Execution · Progress · Results · Cancellation · Failures · Recovery)
├── Examples
└── Notes
```

Copy this file to `skills/<id>/skill.md` and replace placeholders. `vdctl skills validate` requires `## Runtime Contract` plus the six subsections.
