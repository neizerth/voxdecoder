//! `vd-fix-casing` — presentation fixer CLI (see `src/cli/fix/vd-fix-casing/`).

fn main() -> std::process::ExitCode {
    vd_fix_casing::run(std::env::args_os())
}
