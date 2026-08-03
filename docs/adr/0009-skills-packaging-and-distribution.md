# ADR 0009 — Skills Packaging and Distribution

**Status:** Proposed  
**Type:** ADR  
**Date:** 2026-08-03

**Related:** [`vdctl`](../../src/cli/manage/vdctl/) · [`skills/`](../../skills/) · [ADR 0003 — Distribution & Update](0003-distribution-and-update-strategy.md) · [ADR 0005 — MCP Bundle & Skills](0005-mcp-bundle-and-skill-distribution.md) · GitHub Releases

---

## Motivation

Skills have two different lifecycles:

1. **local development** — edit `skill.md`, sync into AI applications, iterate;
2. **end-user distribution** — reproducible packages on GitHub Releases.

These workflows must not share tooling.

Development optimizes for iteration speed.  
Distribution optimizes for packaging and publishing.

ADR 0005 already defines how `vdctl` discovers, validates, synchronizes, and installs Skills locally. This ADR defines how Skills leave the repository as release artifacts — and who is allowed to produce them.

---

## Core rule

```text
vdctl installs and integrates Skills.

Project tooling packages Skills.

Release infrastructure distributes Skills.
```

Each layer owns exactly one responsibility.

---

## Relationship to ADR 0005

| Concern | Owner | ADR |
|---------|-------|-----|
| Skill discovery, validation, sync, MCP Bundle assembly, AI-app install, verify | `vdctl` | 0005 |
| Skill ZIP / Skill-pack generation | project tooling | **0009** |
| Upload of Skill artifacts to GitHub Releases | CI / Release | **0009** (+ 0003 channels) |

ADR 0005 remains authoritative for the **local** Skill lifecycle and the **local** MCP Bundle (`.mcpb`).

This ADR does **not** move Bundle build out of `vdctl`. Building `$VD_HOME/bundles/voxdecoder.mcpb` is local integration, not Skill release packaging.

---

## Responsibilities

### `vdctl`

Owns:

- Skill discovery
- validation (`vdctl skills validate`, …)
- synchronization (`skills/` → `$VD_HOME/skills`)
- MCP Bundle assembly (`.mcpb`)
- installation into supported AI applications
- verification

Never owns:

- Skill ZIP generation
- Skill-pack / DXT generation
- release packaging of Skills
- publishing Skill artifacts to GitHub Releases

### Project tooling

Owns:

- building distributable Skill packages from `skills/`
- ZIP generation (per Skill)
- Skill-pack / DXT generation (aggregated or app-specific)
- preparing release artifacts for CI

Never:

- installs Skills or Bundles into local AI applications
- replaces `vdctl mcp update` for developer iteration

### CI / Release

Owns:

- GitHub Releases (upload)
- artifact publishing
- release validation (checksums, expected file set, …)

Publishing is never a required step of a local developer command.

---

## Development workflow

Workspace layout (source of truth):

```text
skills/
    vd-audio/
        skill.md
    vd-meeting/
        skill.md
```

Release packages normalize to the Agent Skills layout expected by Claude (and similar clients): `SKILL.md` with YAML frontmatter (`name`, `description`). Source `skill.md` is unchanged for `vdctl`.

Developer loop:

```text
edit skill.md
        │
        ▼
vdctl mcp update
        │
        ▼
local AI applications  (+ $VD_HOME/skills, local .mcpb)
```

No ZIPs. No DXT. No Release upload.

---

## Distribution workflow

```text
skills/
    │
    ▼
project package tooling
    │
    ├──► per-Skill ZIP
    └──► Skill pack (DXT / successor)
    │
    ▼
CI validates + uploads
    │
    ▼
GitHub Release
```

Packaging is independent of `vdctl`. A machine without `vdctl` must still be able to produce Skill release artifacts from the repo.

---

## Local MCP Bundle vs Skill release packages

| Artifact | Produced by | Consumed by | Distributable? |
|----------|-------------|-------------|----------------|
| `voxdecoder.mcpb` | `vdctl mcp build` | local AI apps via `vdctl mcp install` | **No** — local integration artifact |
| `vd-audio.zip`, … | project tooling | end-user / AI-app install flows | **Yes** |
| Skill pack (e.g. `voxdecoder-skills.dxt`) | project tooling | end-user / AI-app install flows | **Yes** |

Building a Bundle is distinct from packaging a Release.

### Naming note

Anthropic’s Desktop Extension format historically used `.dxt` and was renamed toward `.mcpb`. This repository already uses `.mcpb` for the **gateway** MCP Bundle (ADR 0005).

Skill release packs must keep unambiguous Release names so they are never mistaken for the local gateway Bundle. Preferred Release names stay explicit (e.g. `voxdecoder-skills.dxt`). Format evolution must not fold Skill packs into `vdctl mcp build`.

---

## Project scripts

Project-level scripts are the developer entry points for packaging. Exact names may evolve; intent is fixed:

```bash
npm run build:skills
```

Prepare / normalize Skill package trees from `skills/` (no install).

```bash
npm run package:skills
```

Produce release artifacts:

```text
ZIP   (per Skill)
DXT   (Skill pack; or successor format)
```

Future:

```bash
npm run release:skills
```

May package + validate + stage artifacts for CI. **Publishing remains CI’s job.**

Suggested home for packaging sources/scripts (when implemented):

```text
packaging/
    mcp/          # existing — local Bundle source (ADR 0005)
    skills/       # Skill release packaging (this ADR)
```

---

## `vdctl` (unchanged install focus)

```text
vdctl mcp update
        │
        ▼
discover → validate → sync Skills → assemble MCP Bundle → install → verify
```

This path never produces Skill release packages.

---

## Distribution formats

Supported Skill release formats (v1 candidates):

```text
ZIP     — one archive per Skill (vd-audio.zip, vd-meeting.zip, …)
DXT     — aggregated / app-oriented Skill pack
```

Additional formats may be added in project tooling and CI without changing `vdctl`.

Users install Release artifacts through their AI application’s supported flow (or by placing extracted Skills where that app expects them). That path is outside `vdctl` packaging.

---

## GitHub Releases

CI uploads Skill artifacts alongside (or as a defined subset of) platform Releases.

Typical layout:

```text
vd-audio.zip
vd-meeting.zip
voxdecoder-skills.dxt
```

Platform binary archives (ADR 0003) and Skill packages are separate artifact classes. Skill packages are not substitutes for Runtime/binary installs.

---

## Separation of concerns

```text
Developer
    → edit skill.md
    → vdctl mcp update
    → local applications
```

```text
Repository
    → package (project tooling)
    → ZIP / DXT
    → GitHub Release (CI)
```

The two workflows remain independent. Crossing them (e.g. `vdctl` emitting Release ZIPs, or `npm run package:skills` writing into AI app config) is out of scope and rejected.

---

## Non-goals

- Changing the Skill Runtime Contract or `skills/TEMPLATE.md` (ADR 0005).
- Moving local MCP Bundle (`.mcpb`) build out of `vdctl`.
- Requiring ZIP generation during everyday Workspace development.
- Making `vdctl` publish to GitHub Releases.
- Defining AI-app-specific install UX beyond “consumer installs Release artifacts via supported flows”.

---

## Consequences

**Positive**

- Clear split: iterate locally with `vdctl`; ship with project tooling + CI.
- New distribution formats do not grow the `vdctl` surface.
- Release packaging becomes reproducible and CI-friendly.

**Trade-offs**

- Two entry points to learn (`vdctl mcp update` vs `npm run package:skills`).
- Skill Release naming must stay distinct from local `.mcpb` Bundles.
- ADR 0005 wording that mixes “distribution” with local install should be read together with this ADR for packaging questions.

---

## Success criteria

- [x] `vdctl` does not own Skill packaging (ZIP / DXT / Skill packs).
- [x] ZIP and DXT (or successors) are produced by project tooling under `packaging/skills/` (or equivalent).
- [x] Local development never requires ZIP / DXT generation.
- [ ] Skill release packaging is reproducible in CI.
- [ ] GitHub Releases contain distributable Skill artifacts with unambiguous names.
- [x] Installation (`vdctl`) and distribution (tooling + CI) remain separate responsibilities.
)
