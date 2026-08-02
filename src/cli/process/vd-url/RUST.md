# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

**Status: v1.** Crate is a workspace member.

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints | `cargo clippy -p vd-url --all-targets -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vd-url --test unit` |
| integration | `tests/integration/` | `cargo test -p vd-url --test integration` |
| e2e | `tests/e2e/` | `cargo test -p vd-url --test e2e` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vd-url` |

Shared: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/), [`vd-output`](../../../crates/vd-output/). Capability leaf: `use: import-url` (bound by Executor / Planning API when implemented).

```bash
# once crate exists:
rustup show
cargo fmt --all
cargo clippy -p vd-url --all-targets -- -D warnings
cargo test -p vd-url
# VD_URL_E2E=1 cargo test -p vd-url --test e2e -- --ignored
```

External tools for e2e:

| Env / binary | Role |
|--------------|------|
| `VD_YTDLP` / `yt-dlp` | YouTube provider |
| `VD_FFMPEG` / `ffmpeg` | Direct video → audio |

Git hooks: [lefthook.yml](../../../../lefthook.yml). `unsafe` forbidden workspace-wide.

Test plan: [STRUCTURE.md § Tests](STRUCTURE.md#tests).
