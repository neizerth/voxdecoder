//! `vd-fix-asr` — ASR wording fixer CLI (see `src/cli/fix/vd-fix-asr/`).

fn main() -> std::process::ExitCode {
    vd_fix_asr::run(std::env::args_os())
}
