# Rust quality gates

**Status: implemented (v1).**

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints | `cargo clippy -p vd-srv --all-targets -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vd-srv --test unit` |
| integration | `tests/integration/` | `cargo test -p vd-srv --test integration` |
| e2e | `tests/e2e/` | `cargo test -p vd-srv --test e2e` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vd-srv` |

Depends on: [`vd-pipeline`](../../process/vd-pipeline/) (Job + Executor), [`vd-progress`](../../../crates/vd-progress/), [`vd-artifact`](../../../crates/vd-artifact/).

```bash
cargo fmt --all
cargo clippy -p vd-srv --all-targets -- -D warnings
cargo test -p vd-srv
```
