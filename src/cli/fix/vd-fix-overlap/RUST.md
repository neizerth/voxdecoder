# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints in root `Cargo.toml` | `cargo clippy --workspace --all-targets -- -D warnings` |
| tests | `src/cli/fix/vd-fix-overlap/tests/{unit,e2e}/` | `cargo test -p vd-fix-overlap` |

**Status: detection only** (see [STRUCTURE.md](STRUCTURE.md)). Workspace member: `vd-fix-overlap`, plus `vd-artifact` for the `paths` helper only.

From repo root:

```bash
rustup show
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vd-fix-overlap
```

Git hooks ([lefthook.yml](../../../../lefthook.yml)): `npm install` installs lefthook; `pre-commit` runs `npm test`; `commit-msg` runs commitlint (conventional commits).

`unsafe` is forbidden workspace-wide. Clippy `pedantic` / `nursery` are on as warnings; CI uses `-D warnings`.

Layout: [STRUCTURE.md](STRUCTURE.md).
