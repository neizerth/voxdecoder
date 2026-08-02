//! `vdctl` CLI entrypoint.

fn main() -> std::process::ExitCode {
    vdctl::run_cli(std::env::args_os())
}
