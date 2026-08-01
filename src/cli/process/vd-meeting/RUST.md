# Rust quality gates

Pinned toolchain: [`rust-toolchain.toml`](../../../../rust-toolchain.toml) (`stable` + `rustfmt` + `clippy` + `rust-analyzer`).

**Status: implemented.**

| Tool | Config | Command |
|------|--------|---------|
| rustfmt | [`rustfmt.toml`](../../../../rustfmt.toml) | `cargo fmt --all -- --check` |
| clippy | [`clippy.toml`](../../../../clippy.toml) + workspace lints in root `Cargo.toml` | `cargo clippy --workspace --all-targets -- -D warnings` |
| unit | `tests/unit/` | `cargo test -p vd-meeting --test unit` |
| integration | `tests/integration/` (`build_job` → Job DAG) | `cargo test -p vd-meeting --test integration` |
| e2e | `tests/e2e/` (binary; full meeting gated) | `cargo test -p vd-meeting --test e2e` |
| scripts | [`scripts/test.sh`](../../../../scripts/test.sh) | `./scripts/test.sh vd-meeting` |

Shared: [`vd-pipeline`](../vd-pipeline/) (Job + Executor), [`src/crates/`](../../../crates/). This crate owns Meeting Model + **MeetingPlanner** only.

From repo root (once the crate exists):

```bash
rustup show
cargo fmt --all
cargo clippy -p vd-meeting --all-targets -- -D warnings
cargo test -p vd-meeting
cargo test -p vd-meeting --test unit
cargo test -p vd-meeting --test integration
cargo test -p vd-meeting --test e2e
# optional full:
# VD_MEETING_E2E_FULL=1 cargo test -p vd-meeting --test e2e -- --ignored
```

Git hooks ([lefthook.yml](../../../../lefthook.yml)): `npm install` installs lefthook; `pre-commit` runs `npm test`; `commit-msg` runs commitlint (conventional commits).

`unsafe` is forbidden workspace-wide. Clippy `pedantic` / `nursery` are on as warnings; CI should use `-D warnings`.

Test plan detail: [STRUCTURE.md § Tests](STRUCTURE.md#tests).
