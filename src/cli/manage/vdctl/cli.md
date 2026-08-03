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
Runtime      up · ensure · down · restart · status
MCP          mcp …
Skills       skills …
Assets       assets …
Diagnostics  health · doctor · info
Config       config …
```

**No `service` verb.** How Runtime is supervised is an implementation detail behind `up` / `ensure` / `down`.

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
vdctl ensure
vdctl down
vdctl restart
vdctl status
vdctl wait [--timeout N]
```

`ensure` — start Runtime only if it is not already reachable (safe for Skills / agents).
`up` — same start path; prints “already running” when reachable.

---

## MCP

```bash
vdctl mcp start | stop | restart | status
vdctl mcp build
vdctl mcp install | update | uninstall | verify | list
vdctl mcp install --apps cursor
vdctl mcp install --apps opencode
vdctl mcp install --apps crush,gemini
vdctl mcp install --skills vd-audio,vd-meeting
vdctl mcp install --exclude vd-meeting
vdctl mcp install --no-skills
vdctl mcp install --dry-run
```

Default `install` (ADR 0005): sync Skills → `$VD_HOME/skills` → build `voxdecoder.mcpb` → install Bundle into AI apps → verify.

`register` / `unregister` retired → use `install` / `uninstall`.

---

## Skills

Platform assets under `skills/<id>/skill.md` → synchronized to `$VD_HOME/skills` by **`vdctl mcp install|update`**. The `skills` command is **read-only** (list / inspect / validate / status). `vd-mcp` never discovers or installs Skills.

Skills that start Jobs must include a `## Runtime Contract` (Execution · Progress · Results · Cancellation · Failures · Recovery). See `skills/TEMPLATE.md`. `vdctl skills validate` enforces this.

After editing `skill.md`:

```bash
vdctl mcp update
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

`discover` shows Applications (Desktop / CLI) + Skills. Adapters: `src/agents/adapters.toml` (`kind = desktop|cli`). Bundle packaging: `packaging/mcp/`.

```bash
vdctl discover --json
# applications[].kind = "desktop" | "cli"
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

[README.md](README.md) · [STRUCTURE.md](STRUCTURE.md) · [ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md) · [ADR 0005](../../../../docs/adr/0005-mcp-bundle-and-skill-distribution.md)
