//! `vd-diarize` CLI — SpeakerTimeline from one audio file.

fn main() -> std::process::ExitCode {
    vd_diarize::run_cli(std::env::args_os())
}
