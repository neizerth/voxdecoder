//! `vd-mcp` CLI entrypoint.

fn main() -> std::process::ExitCode {
    vd_mcp::run_cli(std::env::args_os())
}
