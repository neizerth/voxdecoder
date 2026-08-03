//! `vd-fix-disfluency` — speech disfluency cleanup CLI (see `src/cli/fix/vd-fix-disfluency/`).

fn main() -> std::process::ExitCode {
    vd_fix_disfluency::run(std::env::args_os())
}
