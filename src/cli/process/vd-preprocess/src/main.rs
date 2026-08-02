//! `vd-preprocess` CLI — filter chain → prepared media.

fn main() -> std::process::ExitCode {
    vd_preprocess::run_cli(std::env::args_os())
}
