# Shared crates

Workspace libraries under [`src/crates/`](.) — used by `vd-fix-*` and reusable by other CLIs.

| Crate | Path | Owns |
|-------|------|------|
| **`vd-artifact`** | [`vd-artifact/`](vd-artifact/) | Artifact detect / load / `TextSpan` walk / write; shared types (`ArtifactType`, `Language`, `TextSpan`, …); platform `paths` helpers |
| **`vd-output`** | [`vd-output/`](vd-output/) | `.fixed.` path resolution (`-o` / `-d` / `--in-place` / `--overwrite`) |
| **`vd-progress`** | [`vd-progress/`](vd-progress/) | Stderr progress (`text` \| `json`) + `ProgressFormat` |

Do **not** put presentation / ASR / terms backends here. Those stay in each `vd-fix-*` binary.

```bash
cargo test -p vd-artifact -p vd-output -p vd-progress
```
