//! `vd-url` CLI — online media → ImportResult artifacts.

fn main() -> std::process::ExitCode {
    vd_url::run_cli(std::env::args_os())
}
