# vdctl — Platform Control CLI

**Status:** implemented (v0).

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI surface: [cli.md](cli.md).  
Rust gates: [RUST.md](RUST.md).  
Related: [`vd-srv`](../vd-srv/) · [`vd-mcp`](../vd-mcp/) · [`docs/runtime.md`](../../../../docs/runtime.md) · [ADR 0002](../../../../docs/adr/0002-build-and-container-strategy.md) · [ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md).

---

## Philosophy

```text
vdctl is the manager of the local VoxDecoder installation.
```

Not a binary manager.  
Not a process manager.  
Not a context / profile manager.

It manages the **installation** as a whole: place it on disk, keep it current, start and stop the Runtime that belongs to it, register MCP hosts, manage AI skills, manage assets, and diagnose health.

It never executes transcription, pipelines, or Jobs.

---

## Core rule

```text
vdctl manages the local VoxDecoder installation.

It installs and updates the platform,
starts and stops the Runtime,
manages assets and MCP registration,
and checks health.

It never performs media processing.
```

### Golden Rule

```text
Everything that modifies the platform
goes through vdctl.

Everything that processes user data
goes through the Runtime.

Everything that communicates with AI
goes through Runtime capabilities.
```

### Thin CLI · JSON

Complex logic lives in shared libraries (Desktop uses the same). Any command that **describes state** supports `--json` — Desktop must not scrape human text. See [cli.md](cli.md#output-contract).

---

## Command map

Compact surface — nothing else for the normal user:

```text
Platform        install · update · uninstall
Runtime         up · down · restart · status
MCP             mcp …
Skills          skills …
Assets          assets …
Diagnostics     health · doctor · info
Config          config …
```

Also useful: `wait`, `paths`, `env`, `discover`, `inspect`, `version` — same family as Diagnostics / Platform; details in [cli.md](cli.md).

**No `service` verb.** `vd-srv` is the platform Runtime, not “a systemd unit.” How `up` runs it (foreground process, launchd, Windows Service, Docker attach, already-running) is an implementation detail behind a stable CLI.

---

## Platform

### Install

```bash
vdctl install
```

* download release from GitHub ([ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md));
* install binaries;
* create directories;
* register `vdctl` on PATH where applicable;
* offer MCP host registration.

### Update

```bash
vdctl update
```

* check GitHub Releases;
* download new version;
* update the **whole platform**;
* restart Runtime if needed;
* refresh compatible assets when required;
* run migrations.

### Uninstall

```bash
vdctl uninstall
vdctl uninstall --purge
```

Remove the platform. Default may prompt `Keep data? (Y/n)`; `--purge` removes data too.

---

## Runtime

```bash
vdctl up
vdctl down
vdctl restart
vdctl status
vdctl wait
```

Start / stop / observe the Runtime belonging to this installation. Same commands in Workspace and Installed modes — only binary resolution differs.

---

## MCP

```bash
vdctl mcp start | stop | restart | status
vdctl mcp register | unregister | list
```

Default `mcp register` discovers Skills, detects AI apps, writes MCP config for `vd-mcp`, installs Skills into each app, then verifies. Filters: `--apps`, `--skills`, `--exclude`, `--no-skills`, `--dry-run`.

---

## Skills & AI integration

```text
Runtime  ≠  MCP  ≠  Skills
```

| Component | Responsibility |
|-----------|----------------|
| **skills/** | Skill definitions (`skills/<id>/skill.md`) |
| **vdctl** | Discover, validate, install, update, remove Skills + MCP registration |
| **vd-mcp** | MCP Gateway only (never discovers/installs Skills) |
| **vd-srv** | Runtime |

```bash
vdctl discover
vdctl skills list | inspect <id> | validate | status
vdctl mcp register
vdctl mcp register --apps cursor --skills vd-audio --dry-run
```

Repo layout (no registry file):

```text
skills/
  vd-audio/skill.md
  vd-meeting/skill.md
```

`recipes` process data inside VoxDecoder; `skills` teach a specific AI client how to use VoxDecoder via MCP.

AI adapters: [`adapters.toml`](src/agents/adapters.toml) (+ OS blocks / `skill_dirs` / `mcp_format`).

---

## Assets

```bash
vdctl assets list | install | update | remove
```

Models, packs, filter / diarization assets. Never replaces platform binaries. [ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md).

---

## Diagnostics

```bash
vdctl health
vdctl doctor
vdctl info
```

Plus `paths`, `env`, `discover`, `inspect` when a fuller snapshot is needed (Desktop cold start → prefer `inspect --json`). `discover --json` includes `agents` and `skills`.

Runtime-sourced facts via Runtime API when it is up.

---

## Two sources only

```text
Workspace            developer (Cargo.toml walking up cwd)
Installed Platform   end user / Desktop
```

Docker / Kubernetes do not invent a third mode for `vdctl` — they are how Runtime is deployed elsewhere. Local `vdctl` still manages a local installation or a Workspace.

### Workspace (development)

No `install` / `update` from Releases.

```bash
git pull
vdctl up          # target/debug · cargo build if needed
```

If the user runs `vdctl update` / `vdctl install` / `vdctl uninstall` in a Workspace:

```text
Running from a development workspace.

Updates are managed through Git.
Use:

    git pull
    cargo build
```

Optional: `vdctl dev init` registers **one** workspace path and symlinks the built `vdctl` onto `~/.cargo/bin` (usually already on PATH). Use `vdctl link` to refresh the symlink after rebuild; `vdctl dev init --no-link` skips PATH.

```toml
workspace = "/Users/me/projects/voxdecoder"
auto_build = "missing"    # always | missing | never
auto_start_mcp = false
```

### Installed

```bash
vdctl install
vdctl up
vdctl update
vdctl uninstall
```

No Cargo required. Only **`vdctl`** expected on the global PATH; other binaries stay internal to the install root.

---

## Relationship

```text
            vdctl
               │
   installation / Runtime lifecycle
               │
        ┌──────┴──────┐
        ▼             ▼
     vd-srv        vd-mcp
        │
        ▼
    Runtime API → Executor → Capabilities
```

| Component | Job |
|-----------|-----|
| **`vdctl`** | Local installation manager |
| **`vd-srv`** | Runtime |
| **`vd-mcp`** | MCP Gateway |
| **`vd-pipeline`** | Executor |

---

## Non-goals

* Managing arbitrary binaries or “contexts”
* Job execution / planners / capabilities
* A production `vdctl` container image
* Teaching users `service start` vocabulary

---

## Success criteria

* One command installs the platform.
* One command updates the platform.
* One command uninstalls the platform.
* One command starts / stops the Runtime (`up` / `down`).
* One command validates the installation (`doctor`).
* Only `vdctl` needs to be globally available.
* Workspace and Installed use the same Runtime commands; Platform install/update are Installed-only.

---

## Related

| Doc | Role |
|-----|------|
| [cli.md](cli.md) | Full command surface |
| [STRUCTURE.md](STRUCTURE.md) | Crate layout |
| [ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md) | Distribution & update |
| [`docs/runtime.md`](../../../../docs/runtime.md) | Runtime Environment |
