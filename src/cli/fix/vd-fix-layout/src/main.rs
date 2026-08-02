//! `vd-fix-layout` — layout fixer CLI (see `src/cli/fix/vd-fix-layout/`).

fn main() -> std::process::ExitCode {
    vd_fix_layout::run(std::env::args_os())
}
