//! `vd-gigaam` — GigaAM transcription CLI (see `src/cli/transcribe/vd-gigaam/`).

fn main() -> std::process::ExitCode {
    vd_gigaam::run(std::env::args_os())
}
