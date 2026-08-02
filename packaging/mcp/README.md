# MCP Bundle packaging

Source for the **MCP Bundle** (`.mcpb`) built by `vdctl mcp build` ([ADR 0005](../../docs/adr/0005-mcp-bundle-and-skill-distribution.md)).

```text
packaging/mcp/
  manifest.json     # bundle identity
  icon.png          # optional
```

Output: `$VD_HOME/bundles/voxdecoder.mcpb`

Skills are **not** embedded here — they live under `skills/` → `$VD_HOME/skills`.
