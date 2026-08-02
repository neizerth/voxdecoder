# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| tests (I/O) | `src/crates/{vd-artifact,vd-output}/tests/unit/` | `./scripts/test.sh crates` |
| tests (CLI) | `src/cli/fix/vd-fix-layout/tests/{unit,e2e}/` | `./scripts/test.sh vd-fix-layout` |

Shared crates: [`src/crates/`](../../../crates/).

From repo root (once the crate exists):

```bash
rustup show
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-fix-layout
```

Git hooks: [lefthook.yml](../../../../lefthook.yml). `unsafe` forbidden workspace-wide.

Layout: [STRUCTURE.md](STRUCTURE.md).
