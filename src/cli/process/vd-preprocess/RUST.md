# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

**Status: implemented.**

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints | `cargo clippy -p vd-preprocess --all-targets -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vd-preprocess --test unit` |
| integration | `tests/integration/` | `cargo test -p vd-preprocess --test integration` |
| e2e | `tests/e2e/` | `cargo test -p vd-preprocess --test e2e` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vd-preprocess` |

Shared: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/), [`vd-output`](../../../crates/vd-output/). Bound from [`vd-pipeline`](../vd-pipeline/) as `use: preprocess` (filter-chain executor; not flag soup).

```bash
# once crate exists:
rustup show
cargo fmt --all
cargo clippy -p vd-preprocess --all-targets -- -D warnings
cargo test -p vd-preprocess
# VD_PREPROCESS_E2E_FULL=1 cargo test -p vd-preprocess --test e2e -- --ignored
```

Git hooks: [lefthook.yml](../../../../lefthook.yml). `unsafe` forbidden workspace-wide.

Test plan: [STRUCTURE.md § Tests](STRUCTURE.md#tests).
