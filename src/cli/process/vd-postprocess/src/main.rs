//! `vd-postprocess` CLI — recipes + provider → derived artifacts.

fn main() -> std::process::ExitCode {
    vd_postprocess::run_cli(std::env::args_os())
}
