# Process CLIs

Local document processing **before** the fix pipeline. Prepare project knowledge; do not rewrite transcripts.

```text
pdf / docx / …

        ↓

    vd-assets

        ↓

.voxdecoder/  (md/ + terms.yml)

        ↓

vd-fix-casing → vd-fix-asr → vd-fix-terms
```

| CLI | Role | Spec |
|-----|------|------|
| `vd-assets` | Build reusable project assets (`md/` + `terms.yml`) for `vd-fix-*` | [vd-assets/](vd-assets/) ([cli](vd-assets/cli.md), [structure](vd-assets/STRUCTURE.md)) |

Default project dir: **`.voxdecoder/`** (override `$VD_PROJECT_DIR` or `VD_PROJECT_DIR=` in `.voxdecoder/env` / `.env`). Shared via [`vd-artifact::paths`](../../crates/vd-artifact/).

Fix CLIs: [../fix/README.md](../fix/README.md). Queue / background: [`vd-srv`](../vd-srv/).

```bash
vd-assets run -i ./docs
vd-fix-asr   run -i meeting.txt
vd-fix-terms run -i meeting.txt
```
