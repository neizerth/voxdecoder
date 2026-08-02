# ADR 0005 — MCP Bundle & Skill Distribution

**Status:** Proposed  
**Type:** ADR  
**Date:** 2026-08-02

**Related:** [ADR 0002](0002-build-and-container-strategy.md) · [ADR 0003](0003-distribution-and-update-strategy.md) · [`vdctl`](../../src/cli/manage/vdctl/) · [`vd-mcp`](../../src/cli/manage/vd-mcp/) · [`vd-srv`](../../src/cli/manage/vd-srv/)

---

## Goal

Standardize how VoxDecoder integrates with AI applications.

The platform should install itself into supported AI clients with a single command, without requiring users to manually edit configuration files or understand the MCP protocol.

---

## Core rule

```text
vdctl owns integration with AI applications.

It installs MCP Bundles,
installs Skills,
verifies the installation,
and keeps everything updated.

vd-mcp is only the Runtime Gateway.

vd-srv is the Runtime.
```

---

## Architecture

```text
                  GitHub Release
                        │
                        ▼
                    vdctl install
                        │
        ┌───────────────┼────────────────┐
        ▼               ▼                ▼
 install binaries   install Skills   install MCP Bundle
        │               │                │
        │               │                ▼
        │               │        Claude Desktop
        │               │        Cursor
        │               │        ChatGPT Desktop
        ▼               ▼
     vd-srv        $VD_HOME/skills
        │
        ▼
     Runtime API
        ▲
        │
     vd-mcp
```

---

## Components

| Component | Owns | Never |
|-----------|------|-------|
| **vd-srv** | planning, scheduling, execution, artifacts, events | AI applications |
| **vd-mcp** | MCP protocol, Runtime API client | install itself, discover Skills, edit app config |
| **vdctl** | install / update / uninstall, AI integration, Skill lifecycle, verification | Job execution |
| **MCP Bundle (`.mcpb`)** | launch Gateway inside an AI application | Runtime logic |

---

## MCP Bundle

First-class delivery package for AI applications:

```text
vd-mcp  →  Bundle Builder  →  voxdecoder.mcpb  →  AI client
```

The Bundle is **not** the Runtime and **not** the Gateway. It is the installation package that points the AI client at `vd-mcp`.

Source materials live under `packaging/mcp/` (`manifest.json`, optional `icon.png`).  
`vdctl mcp build` produces `$VD_HOME/bundles/voxdecoder.mcpb`.

---

## Skills

Skills are **platform assets**, not Bundle contents.

```text
$VD_HOME/skills/
    vd-audio/
    vd-meeting/
```

Repository layout mirrors install layout (`skills/<id>/skill.md`).

Skills stay out of the Bundle so they can update independently, be shared across AI apps, and keep the Bundle minimal.

---

## Repository layout

```text
skills/
    vd-audio/skill.md
    vd-meeting/skill.md

packaging/
    mcp/
        manifest.json
        icon.png          # optional
```

---

## Pipelines

### Install

```text
vdctl install
  → binaries → Runtime → Skills → Build Bundle → Install Bundle → Verify
```

### Update

```text
vdctl update
  → binaries → Runtime → Skills → Rebuild Bundle → Reinstall Bundle → Verify
```

Users never edit MCP configuration manually.

---

## CLI

### MCP

```bash
vdctl mcp install | uninstall | build | update | verify | status | list
vdctl mcp start | stop | restart   # gateway process (optional)
```

`install` replaces former `register`. `uninstall` replaces `unregister`.

Filters on install/uninstall: `--apps`, `--skills`, `--exclude`, `--no-skills`, `--dry-run`.

### Skills

```bash
vdctl skills list | inspect | validate | status
```

### Discovery

```bash
vdctl discover [--json]
```

---

## Bundle Builder & adapters

`vdctl` owns the Bundle Builder and one **installer adapter** per AI application (Claude, Cursor, ChatGPT, VS Code, …). Adapters know install location, Bundle/MCP registration, and verification — nothing else. New apps need only a new adapter entry.

---

## Verification

```bash
vdctl mcp verify
```

Checks: Bundle present · Gateway launches · Runtime reachable · API compatible · MCP tools visible · Skills installed.

---

## JSON

Every discovery/state command supports `--json` for Desktop reuse:

```bash
vdctl discover --json
vdctl mcp status --json
vdctl skills list --json
vdctl skills status --json
```

---

## Design principles

* Runtime is independent from AI applications.
* Gateway is independent from installation.
* Skills are independent from Bundles.
* Bundles are independent from Runtime.
* `vdctl` is the only installer.
* Users never edit MCP configuration manually.
* One installation command prepares the complete platform.
* Updating the platform refreshes Runtime, Skills, and Bundles automatically.

---

## Consequences

* `vdctl mcp register` / `unregister` are retired in favor of `install` / `uninstall`.
* Skills catalog is `$VD_HOME/skills` (repo `skills/` is the source in Workspace).
* Desktop and CI consume the same `--json` surfaces.
