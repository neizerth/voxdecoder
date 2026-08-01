# Shared crates

Workspace libraries under [`src/crates/`](.) — used by `vd-fix-*`, `vd-gigaam`, `vd-assets`, and reusable by other CLIs.

| Crate | Path | Owns |
|-------|------|------|
| **`vd-artifact`** | [`vd-artifact/`](vd-artifact/) | Artifact detect / load / `TextSpan` walk / write; shared types; platform `paths` helpers |
| **`vd-output`** | [`vd-output/`](vd-output/) | `-o` / `-d` / `--in-place` / `--overwrite`; caller-supplied file naming |
| **`vd-progress`** | [`vd-progress/`](vd-progress/) | Stderr progress (`start` / `phase` / `done` / `error`) |

Project dictionaries and Office/PDF → Markdown live in the **`vd-assets` CLI** ([`src/cli/process/vd-assets/`](../cli/process/vd-assets/)), not under `src/crates/`.

```bash
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-assets
```
