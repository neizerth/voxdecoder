# vdctl — project layout

**Platform Control CLI.** Thin orchestrator: Workspace vs Installed resolution, lifecycle, assets, updates, discover/inspect. Does **not** execute Jobs.

**Status: implemented (v0).** Path: `src/cli/manage/vdctl`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [`vd-srv`](../vd-srv/) · [`vd-mcp`](../vd-mcp/) · [`docs/runtime.md`](../../../../docs/runtime.md)

---

## Philosophy

```text
Two sources only: Workspace | Installed Platform

vdctl → resolve binaries → lifecycle (vd-srv · vd-mcp)
     → observe via Runtime API
     → never Executor / planners / capabilities
```

| Layer | Knows | Does not know |
|-------|-------|----------------|
| **`vdctl`** | mode detection, which lib to call | Job planning / scheduling |
| **Shared platform libs** | paths, lifecycle, doctor, assets, discover | Desktop UI, MCP protocol details |
| **Runtime API** | public contract | how binaries were built |
| **`vd-srv`** | Runtime | install UX |

**No** first-class multi-context / profile system. Optional single `workspace =` path via `vdctl dev init` only.

---

## Non-goals

- Job / Planner / capability execution inside `vdctl`
- Second Runtime implementation
- A `service` CLI for Runtime (`up` / `down` instead)
- Multi-workspace / multi-environment product surface
- Production Docker image for `vdctl`

---

## Tree (target)

```
src/cli/manage/vdctl/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── error.rs
│   ├── paths.rs
│   ├── cli/
│   ├── config/                  # vdctl.toml · auto_build · auto_start_mcp · optional workspace=
│   ├── resolve/                 # Workspace (cargo metadata) | Installed (install root)
│   ├── lifecycle/               # up / down (no "service" CLI)
│   ├── mcp/                     # process + Bundle build/install/verify (ADR 0005)
│   │   └── bundle.rs
│   ├── agents/                  # AI discovery via adapters.toml (+ mcp_config write)
│   │   └── adapters.toml
│   ├── skills/                  # discover/validate; sync → $VD_HOME/skills
│   ├── client/                  # Runtime API Operator (+ api)
│   ├── doctor/
│   ├── discover/                # discover + inspect (JSON-first; includes agents)
│   ├── assets/
│   ├── update/                  # install · update · uninstall (ADR 0003; Installed only)
│   ├── output/                  # human vs --json
│   └── workflows/               # dev · logs · attach · shell
│
└── tests/
    ├── unit/
    ├── integration/
    ├── e2e/
    └── fixtures/
```

---

## Dependencies (planned)

| Crate | Use |
|-------|-----|
| `vd-srv` (API client types) | Operator |
| shared platform libs | paths, lifecycle, doctor, assets |
| `clap`, `serde`, `toml` | CLI + config |

**Must not** depend on capability crates for media processing.

---

## Wiring (when implemented)

| Hook | Action |
|------|--------|
| Workspace `Cargo.toml` | member `src/cli/manage/vdctl` |
| `scripts/build.sh` | `-p vdctl` |
| `scripts/test.sh` | `vdctl` |
| `package.json` | `build:vdctl` / `install:vdctl` |

---

## Related

[README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md)
