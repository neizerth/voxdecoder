# Rust quality gates

**Status: planned** (crate not yet a workspace member).

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command (when wired) |
|------|--------|----------------------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints | `cargo clippy -p vd-mcp --all-targets -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vd-mcp --test unit` |
| integration | `tests/integration/` | `cargo test -p vd-mcp --test integration` |
| e2e | `tests/e2e/` | `cargo test -p vd-mcp --test e2e` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vd-mcp` |

Planned depends: Runtime API / Transport contract only ([`vd-srv` TRANSPORT](../vd-srv/TRANSPORT.md)). **No** Planner or capability crates for execution. See [STRUCTURE.md](STRUCTURE.md).

```bash
# after `vd-mcp` is added to the workspace
cargo fmt --all
cargo clippy -p vd-mcp --all-targets -- -D warnings
cargo test -p vd-mcp
```

Native vs container builds: [ADR 0002](../../../../docs/adr/0002-build-and-container-strategy.md). Gateway image has no Metal/CUDA needs.
