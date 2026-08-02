# Rust quality gates

**Status: implemented (v0)** — workspace member `vdctl`.

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints | `cargo clippy -p vdctl --all-targets --no-deps -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vdctl --test unit` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vdctl` |

```bash
cargo fmt --all
cargo clippy -p vdctl --all-targets --no-deps -- -D warnings
cargo test -p vdctl
```

v0: Workspace/Installed resolution, `up`/`down`/`wait`/`doctor`/`info`/`discover` (agents + skills), `mcp register`/`unregister`/`list`, `skills list|inspect|validate|status`. Release ops refused in Workspace. Platform `install`/`update`/`uninstall` remain stubs pending [ADR 0003](../../../../docs/adr/0003-distribution-and-update-strategy.md).
