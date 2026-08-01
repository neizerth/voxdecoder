//! `vd-fix-terms` — terminology fixer CLI (see `src/cli/fix/vd-fix-terms/`).

fn main() -> std::process::ExitCode {
    vd_fix_terms::run(std::env::args_os())
}
