# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../clippy.toml) + workspace lints in root `Cargo.toml` | `cargo clippy --workspace --all-targets -- -D warnings` |
| tests | `cli/fix/vd-fix-casing/tests/{unit,e2e}/` | `npm run test:vd-fix-casing` / `cargo test -p vd-fix-casing` |

From repo root:

```bash
rustup show                    # should pick repo toolchain
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vd-fix-casing
```

Git hooks ([lefthook.yml](../../../lefthook.yml)): `npm install` installs lefthook; `pre-commit` runs `npm test`; `commit-msg` runs commitlint (conventional commits).

`unsafe` is forbidden workspace-wide. Clippy `pedantic` / `nursery` are on as warnings; CI should use `-D warnings` so they fail the build.

Layout: [STRUCTURE.md](STRUCTURE.md).
