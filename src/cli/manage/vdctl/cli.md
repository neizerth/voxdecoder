# vdctl CLI surface

**Local installation manager.** Never processes user media. Never competes with `vd-srv` for Jobs.

**Status: implemented (v0).**

Product: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md).  
Runtime: [`vd-srv`](../vd-srv/) · MCP: [`vd-mcp`](../vd-mcp/) · [ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md) · [`docs/runtime.md`](../../../../docs/runtime.md).

---

## Philosophy

```text
vdctl manages the local VoxDecoder installation.
```

Not binaries / processes / contexts — the **installation**.

```text
Platform     install · update · uninstall
Runtime      up · down · restart · status
MCP          mcp …
Skills       skills …
Assets       assets …
Diagnostics  health · doctor · info
Config       config …
```

**No `service` verb.** How Runtime is supervised is an implementation detail behind `up` / `down`.

---

## Modes

```text
Workspace          Cargo.toml walking up cwd → target/debug · no Release install/update
Installed Platform install root → Release install/update/uninstall
```

Same Runtime commands (`up` / `down` / …) in both modes.

---

## Output contract

State commands support `--json`. Desktop must not scrape human text.

---

## Platform

```bash
vdctl install
vdctl update
vdctl update --channel stable
vdctl uninstall
vdctl uninstall --purge
```

In Workspace → refuse with Git/`cargo build` guidance ([README](README.md#workspace-development)).

---

## Runtime

```bash
vdctl up
vdctl down
vdctl restart
vdctl status
vdctl wait [--timeout N]
```

---

## MCP

```bash
vdctl mcp start | stop | restart | status
vdctl mcp register | unregister | list
vdctl mcp register --apps cursor
vdctl mcp register --skills vd-audio,vd-meeting
vdctl mcp register --exclude vd-meeting
vdctl mcp register --no-skills
vdctl mcp register --dry-run
```

Default `register`: discover Skills → detect AI apps → install MCP (`vd-mcp`) → install all Skills → verify.

`unregister` removes MCP + Skills (or `--skills` / `--no-skills` / `--apps` filters). No Runtime restart.

---

## Skills

Platform assets under repo `skills/<id>/skill.md` (directory name = id). **`vdctl` owns the lifecycle**; `vd-mcp` never discovers or installs Skills.

```bash
vdctl skills list [--json]
vdctl skills inspect vd-audio [--json]
vdctl skills validate [--json]
vdctl skills status [--json]
```

```text
Runtime  ≠  MCP  ≠  Skills
```

---

## Assets

```bash
vdctl assets list | install | update | remove
```

---

## Diagnostics

```bash
vdctl health [--json]
vdctl doctor [--json]
vdctl info [--json]
vdctl paths | env | version
vdctl discover | inspect [--json]
```

`discover` shows Applications + Skills. Adapters: built-in `src/agents/adapters.toml`, override `agents.toml` / `VDCTL_AGENTS`. OS blocks `[agent.macos|linux|windows]`.

```bash
vdctl discover --json
```

---

## Config

```bash
vdctl config get | set | edit | path | list
vdctl dev init                 # workspace= + symlink onto ~/.cargo/bin
vdctl link                     # refresh PATH symlink after rebuild
```

---

## Other

```bash
vdctl api …                    # Operator passthrough (debug)
vdctl logs | attach | shell | reset …
```

---

## Non-commands

```text
vdctl run · submit · transcribe · pipeline · service …
```

---

## Related

[README.md](README.md) · [STRUCTURE.md](STRUCTURE.md) · [ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md)
