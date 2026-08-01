//! `vd-giga` — GigaAM transcription CLI (see `cli/transcribe/vd-giga/`).

fn main() -> std::process::ExitCode {
    vd_giga::run(std::env::args_os())
}
