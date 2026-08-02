# Skills

Platform Skill assets for AI clients. Owned by **`vdctl`** — not by `vd-mcp`.

```text
skills/<id>/skill.md     ← required (directory name = Skill id)
skills/<id>/README.md    ← optional
skills/<id>/examples/    ← optional
```

Only directories containing `skill.md` are Skills. Everything else is ignored.

```bash
vdctl skills list
vdctl skills validate
vdctl mcp register          # Runtime MCP + all Skills → all AI apps
vdctl mcp register --dry-run
```

See [`vdctl` README](../src/cli/manage/vdctl/README.md#skills--ai-integration).
