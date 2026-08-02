# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints in root `Cargo.toml` | `cargo clippy --workspace --all-targets -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vd-pipeline --test unit` |
| integration | `tests/integration/` (Executor + stubs) | `cargo test -p vd-pipeline --test integration` |
| e2e | `tests/e2e/` (binary; light runs; full ASR optional) | `cargo test -p vd-pipeline --test e2e` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vd-pipeline` |

Shared crates: [`src/crates/`](../../../crates/). Orchestration only — no domain backends here.

From repo root (once the crate exists):

```bash
rustup show
cargo fmt --all
cargo clippy -p vd-pipeline --all-targets -- -D warnings
cargo test -p vd-pipeline
cargo test -p vd-pipeline --test unit
cargo test -p vd-pipeline --test integration
cargo test -p vd-pipeline --test e2e
# optional full ASR:
# VD_PIPELINE_E2E_FULL=1 cargo test --release -p vd-pipeline --test e2e run_full_pipeline -- --ignored
# experimental preprocess speed vs accuracy:
# VD_PIPELINE_E2E_SPEED=1 VD_PIPELINE_E2E_SPEED_BAND=high \
#   cargo test --release -p vd-pipeline --test e2e preprocess_speed_faster_than_1x -- --ignored --nocapture
# TimeMap remap (1× vs speed 2× segment ends):
# VD_PIPELINE_E2E_TIMEMAP=1 cargo test --release -p vd-pipeline --test e2e \
#   preprocess_speed_2x_timemap_matches_1x_segments -- --ignored --nocapture
```

Git hooks ([lefthook.yml](../../../../lefthook.yml)): `npm install` installs lefthook; `pre-commit` runs `npm test`; `commit-msg` runs commitlint (conventional commits).

`unsafe` is forbidden workspace-wide. Clippy `pedantic` / `nursery` are on as warnings; CI should use `-D warnings`.

Test plan detail: [STRUCTURE.md § Tests](STRUCTURE.md#tests).
