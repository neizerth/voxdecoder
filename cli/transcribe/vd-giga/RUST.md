# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../clippy.toml) + workspace lints in root `Cargo.toml` | `cargo clippy --workspace --all-targets -- -D warnings` |
| tests | `cli/vd-giga/tests/` (when present) | `npm run test:vd-giga` / `npm test` |

From repo root:

```bash
rustup show                    # should pick repo toolchain
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
npm test                       # all CLI packages (see package.json)
npm run test:vd-giga
```

Git hooks ([lefthook.yml](../../../lefthook.yml)): `npm install` installs lefthook; `pre-commit` runs `npm test`; `commit-msg` runs commitlint (conventional commits).

`unsafe` is forbidden workspace-wide. Clippy `pedantic` / `nursery` are on as warnings; CI should use `-D warnings` so they fail the build.
