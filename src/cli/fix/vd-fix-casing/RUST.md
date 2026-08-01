# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints in root `Cargo.toml` | `cargo clippy --workspace --all-targets -- -D warnings` |
| tests (I/O) | `src/crates/{vd-artifact,vd-output}/tests/unit/` | `./scripts/test.sh crates` |
| tests (CLI) | `src/cli/fix/vd-fix-casing/tests/{unit,e2e}/` | `./scripts/test.sh vd-fix-casing` |

Shared crates: [`src/crates/`](../../../crates/). Change artifact / `.fixed.` / progress there — not in this crate.

From repo root:

```bash
rustup show
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vd-artifact -p vd-output -p vd-progress
cargo test -p vd-fix-casing
```

Git hooks ([lefthook.yml](../../../../lefthook.yml)): `npm install` installs lefthook; `pre-commit` runs `npm test`; `commit-msg` runs commitlint (conventional commits).

`unsafe` is forbidden workspace-wide. Clippy `pedantic` / `nursery` are on as warnings; CI should use `-D warnings` so they fail the build.

Layout: [STRUCTURE.md](STRUCTURE.md) (see **Shared crates?**).
