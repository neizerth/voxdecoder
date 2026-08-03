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
vdctl mcp build
vdctl mcp install          # Skills → $VD_HOME/skills + Bundle → AI apps
vdctl mcp install --dry-run
```

Local install / sync: [`vdctl`](../src/cli/manage/vdctl/README.md#skills--ai-integration) · [ADR 0005](../docs/adr/0005-mcp-bundle-and-skill-distribution.md).

Release packages (ZIP / DXT): [`packaging/skills/`](../packaging/skills/) · [ADR 0009](../docs/adr/0009-skills-packaging-and-distribution.md) · `npm run package:skills`.
