//! `vd-fix-overlap` — diarization-overlap duplicate-speech detector CLI
//! (detection only for now — see `src/cli/fix/vd-fix-overlap/STRUCTURE.md`).

fn main() -> std::process::ExitCode {
    vd_fix_overlap::run(std::env::args_os())
}
