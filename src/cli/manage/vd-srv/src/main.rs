//! `vd-srv` CLI — execution engine.

fn main() -> std::process::ExitCode {
    vd_srv::run_cli(std::env::args_os())
}
