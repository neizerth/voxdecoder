# vd-mcp — MCP Builder

**Status:** reserved (not implemented yet)

Layout: sibling of [`vd-srv`](../vd-srv/) under `src/cli/manage/`.  
Platform: [`docs/runtime.md`](../../../../docs/runtime.md) · [ADR 0002](../../../../docs/adr/0002-build-and-container-strategy.md).

MCP is an **interface Builder**, not a Runtime and not an Executor.

```text
Claude Desktop / Cursor / VS Code / …
              ↓
           vd-mcp          (MCP transport: stdio, …)
              ↓
           Transport       (tcp / uds / pipe — same as other clients)
              ↓
           vd-srv          (Runtime — Job lifecycle)
              ↓
           Executor → Capabilities
```

## Why a separate container

| | Runtime (`vd-srv`) | MCP (`vd-mcp`) |
|--|--------------------|----------------|
| Role | Execute & schedule Jobs | Translate MCP ↔ Job submit/observe |
| GPU / models | Yes (via capabilities) | No |
| Required | Core Worker | Optional |
| Wire protocol | Transport (UDS / TCP / …) | MCP to client; Transport to Runtime |

Same Job schema as `vd-pipeline` / `vd-meeting`. Image: `voxdecoder/mcp`.

### Env (Transport — not HTTP)

```text
VD_TRANSPORT=tcp
VD_TCP=runtime:7701

# or
VD_TRANSPORT=uds
VD_SOCKET=/tmp/vd.sock
```

Until this crate exists, `docker build --target mcp` ships a stub that exits with these knobs printed.
