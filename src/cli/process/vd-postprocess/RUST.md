# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

**Status: implemented.**

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vd-postprocess --test unit` |
| integration | `tests/integration/` | `cargo test -p vd-postprocess --test integration` |
| e2e | `tests/e2e/` | `cargo test -p vd-postprocess --test e2e` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vd-postprocess` |

Shared: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/). Bound from [`vd-pipeline`](../vd-pipeline/) as `use: postprocess` (recipe executor; not LLM-only).

```bash
# once crate exists:
rustup show
cargo fmt --all
cargo clippy -p vd-postprocess --all-targets -- -D warnings
cargo test -p vd-postprocess
# VD_POSTPROCESS_E2E_FULL=1 cargo test -p vd-postprocess --test e2e -- --ignored
```

Git hooks: [lefthook.yml](../../../../lefthook.yml). `unsafe` forbidden workspace-wide.

Test plan: [STRUCTURE.md § Tests](STRUCTURE.md#tests).
