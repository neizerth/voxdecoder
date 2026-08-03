# Skill release packaging

Builds **distributable** Skill artifacts for GitHub Releases ([ADR 0009](../../docs/adr/0009-skills-packaging-and-distribution.md)).

Does **not** install into AI applications — that is [`vdctl`](../../src/cli/manage/vdctl/) ([ADR 0005](../../docs/adr/0005-mcp-bundle-and-skill-distribution.md)).

```text
skills/                  ← source of truth
    │
    ▼
packaging/skills/        ← this directory
    │
    ▼
dist/skills/
    vd-audio.zip
    vd-meeting.zip
    voxdecoder-skills.dxt
```

## Commands

```bash
npm run build:skills      # stage trees under dist/skills/staging
npm run package:skills    # ZIP + DXT under dist/skills
./packaging/skills/package.sh clean
```

## Artifacts

| File | Contents |
|------|----------|
| `<id>.zip` | Single Skill directory (`<id>/SKILL.md`, …) |
| `voxdecoder-skills.dxt` | Skill pack: `manifest.json` + `skills/<id>/…` |

`.dxt` here is a **Skill pack**, not the local MCP gateway Bundle (`.mcpb` from [`packaging/mcp/`](../mcp/)).

## Normalization

Repo source stays `skills/<id>/skill.md` for `vdctl` (ADR 0005). Packaging emits **Claude / Agent Skills** layout:

- file name: `SKILL.md`
- YAML frontmatter: `name` (skill id) + `description` (from `## Purpose`, ≤200 chars)

```text
vd-audio.zip
└── vd-audio/
    └── SKILL.md    ← --- / name / description / --- + body
```

## Layout

```text
packaging/skills/
  manifest.json   # Skill-pack identity
  package.sh      # build | package | clean
  README.md
```

Publishing to GitHub Releases remains CI’s job.
